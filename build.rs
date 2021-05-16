use std::env;

fn main() {
    println!(
        "cargo:rustc-env=DISCORD_APPLICATION_ID={}",
        env::var("DISCORD_APPLICATION_ID").unwrap()
    );
}
