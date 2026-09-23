//! `xrpl tx fields <Type>` — what a transaction type accepts.
//!
//! A listing command rather than a pipeline stage, so unlike the stages it does
//! take `--json`. That JSON is also what generates the checked-in surface
//! snapshot, so there is one rendering path rather than two that can disagree.

use serde_json::{json, Value};

use crate::commands::tx::txdef;
use crate::error::Error;
use crate::output;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The transaction type. Omit to list every type.
    #[arg(value_name = "TYPE")]
    pub tx_type: Option<String>,

    /// Emit the table as JSON.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let Some(name) = &self.tx_type else {
            return self.list_types();
        };

        let definition = txdef::transaction(name)
            .ok_or_else(|| Error::other(format!("unknown transaction type {name:?}")))?;

        if self.json {
            return output::artifact(&describe(definition));
        }

        println!("{} (xrpl tx new {})", definition.name, definition.command);
        println!("\nFields:");
        for field in definition.generated_fields() {
            let required = if field.required { " (required)" } else { "" };
            println!(
                "  --{:<28} {:<12}{}",
                field.flag, field.serialization_type, required
            );
        }

        if !definition.flags.is_empty() {
            println!("\nFlags (--flag NAME):");
            for (flag, bit) in &definition.flags {
                println!("  {flag:<28} {bit}");
            }
        }

        Ok(())
    }

    fn list_types(&self) -> Result<(), Error> {
        if self.json {
            let names: Vec<&str> = txdef::transactions().iter().map(|tx| tx.name).collect();
            return output::artifact(&names);
        }

        for definition in txdef::transactions() {
            println!("{:<34} xrpl tx new {}", definition.name, definition.command);
        }

        Ok(())
    }
}

/// One transaction type, as JSON.
pub fn describe(definition: &txdef::TransactionDef) -> Value {
    json!({
        "name": definition.name,
        "command": definition.command,
        "fields": definition.generated_fields().map(|field| json!({
            "name": field.name,
            "flag": field.flag,
            "type": field.serialization_type,
            "required": field.required,
        })).collect::<Vec<_>>(),
        "flags": definition.flags.iter().map(|(name, bit)| json!({
            "name": name,
            "bit": bit,
        })).collect::<Vec<_>>(),
    })
}

/// The whole surface, as Markdown.
///
/// Checked in as `xrpl-cli/TX_SURFACE.md` and diffed in CI. A generated command
/// surface is otherwise un-reviewable: nothing in a pull request shows that a
/// field changed type or stopped being required.
pub fn render_markdown() -> String {
    let mut out = String::from(
        "# `xrpl tx new` surface\n\n\
         Generated from the vendored `definitions.json`. Regenerate with\n\
         `cargo run -p xrpl-cli --bin tx-surface > xrpl-cli/TX_SURFACE.md`;\n\
         CI fails if this file and the definitions disagree.\n\n",
    );

    for definition in txdef::transactions() {
        out.push_str(&format!(
            "## {} — `xrpl tx new {}`\n\n| flag | type | required |\n|---|---|---|\n",
            definition.name, definition.command
        ));

        for field in definition.generated_fields() {
            out.push_str(&format!(
                "| `--{}` | {} | {} |\n",
                field.flag,
                field.serialization_type,
                if field.required { "yes" } else { "" }
            ));
        }

        if !definition.flags.is_empty() {
            out.push_str("\nFlags: ");
            let names: Vec<String> = definition
                .flags
                .iter()
                .map(|(name, bit)| format!("`{name}` ({bit})"))
                .collect();
            out.push_str(&names.join(", "));
            out.push('\n');
        }

        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_markdown_covers_every_type() {
        let rendered = render_markdown();

        for definition in txdef::transactions() {
            assert!(
                rendered.contains(&format!("## {} —", definition.name)),
                "{} missing from the surface snapshot",
                definition.name
            );
        }
    }

    #[test]
    fn test_describe_is_the_same_data_as_the_table() {
        let payment = txdef::transaction("Payment").expect("Payment");
        let described = describe(payment);

        assert_eq!(described["name"], "Payment");
        assert_eq!(
            described["fields"].as_array().expect("fields").len(),
            payment.generated_fields().count()
        );
    }
}
