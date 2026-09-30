use frizbee::{CaseMatching, Config, Matcher, Matching, SortStrategy, UnicodeMatching};
use mlua::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parse options table from Lua into a frizbee Config and extra options.
struct MatchOpts {
    config: Config,
    limit: Option<usize>,
    with_positions: bool,
}

impl MatchOpts {
    fn from_lua_opt(opts: Option<LuaTable>) -> LuaResult<Self> {
        let mut result = Self {
            config: Config {
                max_typos: Some(0),
                sort: SortStrategy::ScoreThenIndexAsc,
                ..Config::default()
            },
            limit: None,
            with_positions: false,
        };

        let Some(opts) = opts else {
            return Ok(result);
        };

        if let Some(max_typos) = opts.get::<Option<u16>>("max_typos")? {
            result.config.max_typos = Some(max_typos);
        }
        if let Some(casing) = opts.get::<Option<LuaString>>("casing")? {
            let casing = casing.to_str()?;
            result.config.casing = parse_casing(&casing)?;
        }
        if let Some(unicode) = opts.get::<Option<LuaString>>("unicode")? {
            let unicode = unicode.to_str()?;
            result.config.unicode = parse_unicode(&unicode)?;
        }
        if let Some(matching) = opts.get::<Option<LuaString>>("matching")? {
            let matching = matching.to_str()?;
            result.config.matching = parse_matching(&matching)?;
        }
        if let Some(sort) = opts.get::<Option<LuaValue>>("sort")? {
            result.config.sort = parse_sort(sort)?;
        }
        if let Some(limit) = opts.get::<Option<usize>>("limit")? {
            if limit == 0 {
                return Err(LuaError::runtime("opts.limit must be > 0"));
            }
            result.limit = Some(limit);
        }
        if let Some(with_positions) = opts.get::<Option<bool>>("with_positions")? {
            result.with_positions = with_positions;
        }

        Ok(result)
    }
}

fn parse_casing(value: &str) -> LuaResult<CaseMatching> {
    match value {
        "smart" => Ok(CaseMatching::Smart),
        "ignore" => Ok(CaseMatching::Ignore),
        "respect" => Ok(CaseMatching::Respect),
        other => Err(LuaError::runtime(format!(
            "invalid opts.casing: {other:?} (expected 'smart', 'ignore', or 'respect')"
        ))),
    }
}

fn parse_unicode(value: &str) -> LuaResult<UnicodeMatching> {
    match value {
        "smart" => Ok(UnicodeMatching::Smart),
        "ignore" => Ok(UnicodeMatching::Ignore),
        "always" => Ok(UnicodeMatching::Always),
        other => Err(LuaError::runtime(format!(
            "invalid opts.unicode: {other:?} (expected 'smart', 'ignore', or 'always')"
        ))),
    }
}

fn parse_matching(value: &str) -> LuaResult<Matching> {
    match value {
        "fuzzy" => Ok(Matching::Fuzzy),
        "exact" => Ok(Matching::Exact),
        "prefix" => Ok(Matching::Prefix),
        "suffix" => Ok(Matching::Suffix),
        "substring" => Ok(Matching::Substring),
        other => Err(LuaError::runtime(format!(
            "invalid opts.matching: {other:?} (expected 'fuzzy', 'exact', 'prefix', 'suffix', or 'substring')"
        ))),
    }
}

fn parse_sort(value: LuaValue) -> LuaResult<SortStrategy> {
    match value {
        LuaValue::String(s) => {
            let s = s.to_str()?;
            match &*s {
                "score" => Ok(SortStrategy::ScoreThenIndexAsc),
                "score_reverse" => Ok(SortStrategy::ScoreThenIndexDesc),
                "index" => Ok(SortStrategy::IndexAsc),
                "index_reverse" => Ok(SortStrategy::IndexDesc),
                other => Err(LuaError::runtime(format!(
                    "invalid opts.sort: {other:?} (expected 'score', 'score_reverse', 'index', or 'index_reverse')"
                ))),
            }
        }
        other => Err(LuaError::runtime(format!(
            "invalid opts.sort of type {} (expected a string)",
            other.type_name()
        ))),
    }
}

/// `frizbee_nvim.match(query, texts, opts?) -> matches`
///
/// Fuzzy-match `query` against an array of `texts` and return ranked results.
///
/// Returns an array of tables:
///   { index = <1-based>, score = <number>, exact = <bool>, positions = int[]|nil }
///
/// When `query` is empty, returns all items with score 0, capped by `limit`.
fn lua_match(
    lua: &Lua,
    (query, texts, opts): (String, Vec<String>, Option<LuaTable>),
) -> LuaResult<LuaTable> {
    let match_opts = MatchOpts::from_lua_opt(opts)?;
    let mut matcher = Matcher::new(query.as_str(), &match_opts.config);

    let result = lua.create_table()?;

    if match_opts.with_positions {
        let matches = matcher.match_list_indices(&texts);

        let limit = match_opts.limit.unwrap_or(matches.len());
        for (i, m) in matches.into_iter().take(limit).enumerate() {
            let entry = lua.create_table()?;
            // Convert to 1-based index for Lua
            entry.set("index", m.index + 1)?;
            entry.set("score", m.score)?;
            entry.set("exact", m.exact)?;

            // Convert indices to 1-based Lua array
            let positions = lua.create_table()?;
            for (j, &pos) in m.indices.iter().enumerate() {
                positions.set(j + 1, pos + 1)?;
            }
            entry.set("positions", positions)?;

            result.set(i + 1, entry)?;
        }
    } else {
        let matches = matcher.match_list(&texts);

        let limit = match_opts.limit.unwrap_or(matches.len());
        for (i, m) in matches.into_iter().take(limit).enumerate() {
            let entry = lua.create_table()?;
            entry.set("index", m.index + 1)?;
            entry.set("score", m.score)?;
            entry.set("exact", m.exact)?;
            result.set(i + 1, entry)?;
        }
    }

    Ok(result)
}

/// `frizbee_nvim.match_indices(query, text, opts?) -> integer[]|nil`
///
/// Return 1-based match positions for a single query/text pair, or nil if no match.
fn lua_match_indices(
    lua: &Lua,
    (query, text, opts): (String, String, Option<LuaTable>),
) -> LuaResult<LuaValue> {
    let match_opts = MatchOpts::from_lua_opt(opts)?;

    if query.is_empty() {
        return Ok(LuaValue::Nil);
    }

    let mut matcher = Matcher::new(query.as_str(), &match_opts.config);

    match matcher.match_one_indices(text.as_str(), 0) {
        Some(m) => {
            let positions = lua.create_table()?;
            for (j, &pos) in m.indices.iter().enumerate() {
                positions.set(j + 1, pos + 1)?;
            }
            Ok(LuaValue::Table(positions))
        }
        None => Ok(LuaValue::Nil),
    }
}

/// `frizbee_nvim.health() -> table`
///
/// Returns module metadata for health checks.
fn lua_health(lua: &Lua, _: ()) -> LuaResult<LuaTable> {
    let info = lua.create_table()?;
    info.set("backend", "rust")?;
    info.set("module", "frizbee_nvim")?;
    info.set("version", VERSION)?;

    let features = lua.create_table()?;
    features.set(1, "fuzzy_match")?;
    features.set(2, "match_indices")?;
    features.set(3, "casing")?;
    features.set(4, "matching_modes")?;
    features.set(5, "unicode_matching")?;
    features.set(6, "sort_strategies")?;
    features.set(7, "typo_tolerance")?;
    info.set("features", features)?;

    Ok(info)
}

#[mlua::lua_module]
fn frizbee_nvim(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    exports.set("match", lua.create_function(lua_match)?)?;
    exports.set("match_indices", lua.create_function(lua_match_indices)?)?;
    exports.set("health", lua.create_function(lua_health)?)?;

    Ok(exports)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sort_strategy(sort: bool) -> SortStrategy {
        if sort {
            SortStrategy::ScoreThenIndexAsc
        } else {
            SortStrategy::IndexAsc
        }
    }

    // Helper: run match_list directly (no Lua) to test the core logic paths
    fn do_match(
        query: &str,
        texts: &[&str],
        max_typos: Option<u16>,
        sort: bool,
    ) -> Vec<frizbee::Match> {
        let config = Config {
            max_typos,
            sort: sort_strategy(sort),
            ..Config::default()
        };
        Matcher::new(query, &config).match_list(texts)
    }

    fn do_match_indices(
        query: &str,
        texts: &[&str],
        max_typos: Option<u16>,
    ) -> Vec<frizbee::MatchIndices> {
        let config = Config {
            max_typos,
            sort: SortStrategy::ScoreThenIndexAsc,
            ..Config::default()
        };
        Matcher::new(query, &config).match_list_indices(texts)
    }

    // --- Basic match ordering ---

    #[test]
    fn test_basic_ordering() {
        let texts = ["fooBar", "foo_bar", "prelude", "println!"];
        let matches = do_match("fBr", &texts, Some(0), true);

        // Should match fooBar and foo_bar but not prelude/println
        assert!(!matches.is_empty());
        // Best match should be first (highest score)
        for i in 1..matches.len() {
            assert!(
                matches[i - 1].score >= matches[i].score,
                "results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_exact_match_ranked_first() {
        let texts = ["foobar", "foo", "foob", "foobart"];
        let matches = do_match("foo", &texts, Some(0), true);

        assert!(!matches.is_empty());
        // The exact match "foo" at index 1 should be first
        assert_eq!(matches[0].index, 1);
        assert!(matches[0].exact);
    }

    // --- Tie-break determinism ---

    #[test]
    fn test_tiebreak_determinism() {
        // Identical strings should be tied on score; tie-break by smaller index
        let texts = ["alpha", "alpha", "alpha"];
        let matches = do_match("alpha", &texts, Some(0), true);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].index, 0);
        assert_eq!(matches[1].index, 1);
        assert_eq!(matches[2].index, 2);

        // All should have the same score
        assert_eq!(matches[0].score, matches[1].score);
        assert_eq!(matches[1].score, matches[2].score);
    }

    #[test]
    fn test_determinism_across_runs() {
        let texts = [
            "src/main.rs",
            "src/lib.rs",
            "src/utils/mod.rs",
            "Cargo.toml",
        ];
        let r1 = do_match("src", &texts, Some(0), true);
        let r2 = do_match("src", &texts, Some(0), true);

        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.index, b.index);
            assert_eq!(a.score, b.score);
        }
    }

    // --- Limit behavior ---

    #[test]
    fn test_limit_truncates() {
        let texts: Vec<String> = (0..100).map(|i| format!("item_{i}")).collect();
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let matches = do_match("item", &refs, Some(0), true);

        assert_eq!(matches.len(), 100);
        // Simulating limit=5 (our Lua layer applies limit via .take())
        let limited: Vec<_> = matches.into_iter().take(5).collect();
        assert_eq!(limited.len(), 5);
    }

    #[test]
    fn test_limit_larger_than_results() {
        let texts = ["foo", "bar"];
        let matches = do_match("foo", &texts, Some(0), true);

        // Only "foo" matches; limit>result count should be fine
        let limited: Vec<_> = matches.into_iter().take(100).collect();
        assert_eq!(limited.len(), 1);
    }

    // --- max_typos ---

    #[test]
    fn test_max_typos_zero_strict() {
        let texts = ["hello", "helo", "world"];
        let matches = do_match("hello", &texts, Some(0), true);

        // With 0 typos, "helo" (missing one char) should not match
        // Only "hello" should match
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].index, 0);
    }

    #[test]
    fn test_max_typos_allows_missing() {
        let texts = ["hello", "helo", "world"];
        let matches = do_match("hello", &texts, Some(1), true);

        // With 1 typo tolerance, "helo" should also match
        assert!(matches.len() >= 2);
        let indices: Vec<u32> = matches.iter().map(|m| m.index).collect();
        assert!(indices.contains(&0)); // "hello"
        assert!(indices.contains(&1)); // "helo"
    }

    #[test]
    fn test_max_typos_none_unlimited() {
        let texts = ["abcdef", "xyz", "abc"];
        let matches = do_match("abcdef", &texts, None, true);

        // Unlimited typos should match everything
        assert_eq!(matches.len(), 3);
    }

    // --- match_indices correctness ---

    #[test]
    fn test_match_indices_basic() {
        let texts = ["fooBar"];
        let matches = do_match_indices("fBr", &texts, Some(0));

        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        assert_eq!(m.index, 0);
        assert!(!m.indices.is_empty());

        // All positions must be valid indices into "fooBar"
        for &pos in &m.indices {
            assert!((pos as usize) < "fooBar".len());
        }
    }

    #[test]
    fn test_match_indices_exact() {
        let texts = ["foo"];
        let matches = do_match_indices("foo", &texts, Some(0));

        assert_eq!(matches.len(), 1);
        assert!(matches[0].exact);
        // For exact match "foo", positions should be [0, 1, 2]
        let mut positions = matches[0].indices.clone();
        positions.sort();
        assert_eq!(positions, vec![0, 1, 2]);
    }

    #[test]
    fn test_match_indices_no_match() {
        let texts = ["xyz"];
        let matches = do_match_indices("abc", &texts, Some(0));

        assert!(matches.is_empty());
    }

    // --- Empty query ---

    #[test]
    fn test_empty_query_returns_all() {
        let texts = ["alpha", "beta", "gamma"];
        let matches = do_match("", &texts, Some(0), true);

        // Empty query: return all items with score 0
        assert_eq!(matches.len(), 3);
        for m in &matches {
            assert_eq!(m.score, 0);
            assert!(!m.exact);
        }
    }

    #[test]
    fn test_empty_texts_returns_empty() {
        let texts: Vec<&str> = vec![];
        let matches = do_match("query", &texts, Some(0), true);
        assert!(matches.is_empty());
    }

    // --- Case sensitivity ---

    #[test]
    fn test_casing_ignore_matches_all_variants() {
        let config = Config {
            max_typos: Some(0),
            casing: CaseMatching::Ignore,
            ..Config::default()
        };
        let matches = Matcher::new("FOO", &config).match_list(&["FooBar", "foobar", "FOOBAR"]);
        assert_eq!(
            matches.len(),
            3,
            "case-insensitive should match all variants"
        );
    }

    #[test]
    fn test_casing_respect_excludes_wrong_case() {
        let config = Config {
            max_typos: Some(0),
            casing: CaseMatching::Respect,
            sort: SortStrategy::IndexAsc,
            ..Config::default()
        };
        let matches = Matcher::new("foo", &config).match_list(&["foo", "FOO", "fOo", "xxfooxx"]);
        let indices: Vec<u32> = matches.iter().map(|m| m.index).collect();
        assert_eq!(indices, vec![0, 3]);
    }

    #[test]
    fn test_casing_smart_respects_uppercase_needle() {
        let config = Config {
            max_typos: Some(0),
            casing: CaseMatching::Smart,
            sort: SortStrategy::IndexAsc,
            ..Config::default()
        };
        let matches = Matcher::new("FoO", &config).match_list(&["foo", "FOO", "FoO", "xxFoOxx"]);
        let indices: Vec<u32> = matches.iter().map(|m| m.index).collect();
        assert_eq!(indices, vec![2, 3]);
    }

    // --- Matching modes ---

    #[test]
    fn test_matching_prefix() {
        let config = Config {
            matching: Matching::Prefix,
            ..Config::default()
        };
        let matches = Matcher::new("foo", &config).match_list(&["foobar", "barfoo", "foo"]);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_matching_substring() {
        let config = Config {
            matching: Matching::Substring,
            ..Config::default()
        };
        let matches = Matcher::new("foo", &config).match_list(&["xxfoo", "bar", "foo"]);
        assert_eq!(matches.len(), 2);
    }

    // --- Sort option ---

    #[test]
    fn test_sort_false_preserves_match_order() {
        let texts = ["zzz_foo", "aaa_foo", "mmm_foo"];
        let unsorted = do_match("foo", &texts, Some(0), false);
        let sorted = do_match("foo", &texts, Some(0), true);

        // Both should have same items
        assert_eq!(unsorted.len(), sorted.len());
        // Unsorted (IndexAsc) preserves input order
        let indices: Vec<u32> = unsorted.iter().map(|m| m.index).collect();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    // --- Large input ---

    #[test]
    fn test_large_candidate_list() {
        let texts: Vec<String> = (0..10_000).map(|i| format!("candidate_{i}")).collect();
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let matches = do_match("cand", &refs, Some(0), true);
        assert!(!matches.is_empty());
        // Should complete without pathological slowdown (tested by running)
    }
}
