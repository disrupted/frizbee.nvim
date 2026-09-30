# frizbee.nvim

High-performance fuzzy matching for Neovim, powered by the Rust [frizbee](https://github.com/saghen/frizbee) library.

The plugin consists of:

- A native Rust backend (`frizbee_nvim`) compiled to a shared library.
- A Lua wrapper (`require('frizbee')`) that loads the backend automatically
  and exposes a stable API.

## Installation

### lazy.nvim

```lua
{
  "disrupted/frizbee.nvim",
  build = "cargo build --release",
}
```

The backend is loaded **lazily** on first call; there is no startup cost.

## API

```lua
local frizbee = require('frizbee')
```

### `frizbee.match(query, texts, opts?) -> matches`

Fuzzy-match `query` against an array of strings. Returns ranked results.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `query` | `string` | The search query |
| `texts` | `string[]` | Array of candidate strings |
| `opts` | `table\|nil` | Options (see below) |

**Options**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_typos` | `integer\|nil` | `0` | Max allowed missing chars. `nil` = unlimited. |
| `limit` | `integer\|nil` | `nil` | Max results to return. `nil` = all. |
| `sort` | `string\|nil` | `"score"` | Result ordering. See below. |
| `with_positions` | `boolean\|nil` | `false` | Include match positions. |
| `casing` | `string\|nil` | `"smart"` | `"smart"` (case-insensitive unless the query has uppercase), `"ignore"`, or `"respect"`. |
| `matching` | `string\|nil` | `"fuzzy"` | `"fuzzy"`, `"exact"`, `"prefix"`, `"suffix"`, or `"substring"`. Literal modes ignore `max_typos`. |
| `unicode` | `string\|nil` | `"smart"` | `"smart"`, `"ignore"`, or `"always"`. |

**`sort` values**

| Value | Ordering |
|-------|----------|
| `"score"` | Score descending, then input index ascending |
| `"score_reverse"` | Score descending, then input index descending |
| `"index"` | Input order |
| `"index_reverse"` | Reverse input order |

**Returns** `table[]` — array of match objects:

| Field | Type | Description |
|-------|------|-------------|
| `index` | `integer` | 1-based index into `texts` |
| `score` | `number` | Match score (higher = better) |
| `exact` | `boolean` | Whether the match is exact |
| `positions` | `integer[]\|nil` | 1-based positions in reverse match order (only with `with_positions = true`) |

**Empty query:** returns all items with `score = 0` and `exact = false`,
capped by `limit`.

**Example**

```lua
local matches = frizbee.match("src", {
  "src/main.rs",
  "src/lib.rs",
  "Cargo.toml",
  "README.md",
}, { limit = 10 })

for _, m in ipairs(matches) do
  print(m.index, m.score, m.exact)
end
```


### `frizbee.match_indices(query, text, opts?) -> integer[]|nil`

Return 1-based match positions for a single query/text pair.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `query` | `string` | The search query |
| `text` | `string` | A single candidate string |
| `opts` | `table\|nil` | Same options as `match()` (only `max_typos`, `casing`, `matching`, and `unicode` are used). |

**Returns** `integer[]` of 1-based positions, or `nil` if no match or empty
query.

**Example**

```lua
local pos = frizbee.match_indices("fb", "fooBar")
-- pos = { 4, 1 }  (reverse match order)
```

---

## Troubleshooting

```vim
:checkhealth frizbee
```

Reports platform, library extension, searched paths, and native module status.
