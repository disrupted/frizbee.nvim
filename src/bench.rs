use std::time::{Duration, Instant};

use frizbee::{Config, match_list, match_list_indices};

fn bench<F: FnMut()>(label: &str, iterations: u32, mut f: F) {
    // Warmup
    for _ in 0..3 {
        f();
    }

    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        total += elapsed;
        min = min.min(elapsed);
        max = max.max(elapsed);
    }

    let avg = total / iterations;
    println!(
        "{label:40} avg={avg:>10.3?}  min={min:>10.3?}  max={max:>10.3?}  ({iterations} iters)"
    );
}

fn generate_candidates(n: usize) -> Vec<String> {
    let paths = [
        "src/main.rs",
        "src/lib.rs",
        "src/utils/helpers.rs",
        "src/models/user.rs",
        "src/controllers/auth.rs",
        "tests/integration/test_auth.rs",
        "Cargo.toml",
        "README.md",
        "docs/api/reference.md",
        "lua/telescope/init.lua",
        "lua/telescope/pickers.lua",
        "lua/telescope/finders.lua",
        "node_modules/react/index.js",
        "package.json",
        "tsconfig.json",
        ".github/workflows/ci.yml",
        "docker/Dockerfile",
        "scripts/deploy.sh",
        "config/settings.toml",
        "migrations/001_create_users.sql",
    ];

    (0..n)
        .map(|i| {
            let base = paths[i % paths.len()];
            if i < paths.len() {
                base.to_string()
            } else {
                format!("project_{}/{base}", i / paths.len())
            }
        })
        .collect()
}

fn main() {
    println!("frizbee-nvim benchmark");
    println!("======================\n");

    let queries = ["src", "main", "tele", "auth", "config", "fb"];
    let sizes = [500, 2_000, 5_000, 10_000, 50_000];
    let iterations = 50;

    let config_no_typos = Config {
        max_typos: Some(0),
        sort: true,
        ..Config::default()
    };

    let config_with_typos = Config {
        max_typos: Some(1),
        sort: true,
        ..Config::default()
    };

    for &size in &sizes {
        let candidates = generate_candidates(size);
        let refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();

        println!("--- {size} candidates ---");

        for &query in &queries {
            bench(&format!("match(\"{query}\", {size})"), iterations, || {
                let _ = match_list(query, &refs, &config_no_typos);
            });
        }

        // Bench with positions
        bench(
            &format!("match_indices(\"src\", {size})"),
            iterations,
            || {
                let _ = match_list_indices("src", &refs, &config_no_typos);
            },
        );

        // Bench with typo tolerance
        bench(
            &format!("match(\"src\", {size}, typos=1)"),
            iterations,
            || {
                let _ = match_list("src", &refs, &config_with_typos);
            },
        );

        println!();
    }
}
