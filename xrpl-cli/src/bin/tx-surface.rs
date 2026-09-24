//! Regenerates `xrpl-cli/TX_SURFACE.md`.
//!
//! A generated command surface is otherwise invisible to review: nothing in a
//! pull request shows that a field changed type or stopped being required. CI
//! regenerates this and fails on a diff.

fn main() {
    print!("{}", xrpl_cli::commands::tx::fields::render_markdown());
}
