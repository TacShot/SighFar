use std::io::{self, Write};

pub struct TerminalUi;

impl TerminalUi {
    pub fn render_header(&self) {
        self.clear_screen();
        println!("\x1b[31m====================================================================\x1b[0m");
        println!("\x1b[31;1m  SighFar // offline cipher workbench\x1b[0m");
        println!("\x1b[31m====================================================================\x1b[0m");
        println!();
        println!("  [1] Encode message");
        println!("  [2] Decode message");
        println!("  [3] View encrypted history");
        println!("  [4] Settings");
        println!("  [0] Quit");
        println!();
    }

    pub fn print_panel(&self, title: &str, body: &str) {
        println!("\x1b[31;1m[ {title} ]\x1b[0m");
        println!("{body}");
        println!();
    }

    pub fn prompt(&self, label: &str) -> io::Result<String> {
        print!("{label} ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    pub fn pause(&self) -> io::Result<()> {
        let _ = self.prompt("Press return to continue...")?;
        Ok(())
    }

    pub fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }
}
