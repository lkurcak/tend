use colored::{ColoredString, Colorize};

pub trait TendColors {
    fn job(&self) -> ColoredString;
    fn program(&self) -> ColoredString;
    fn success(&self) -> ColoredString;
    fn failure(&self) -> ColoredString;
}

impl TendColors for str {
    fn job(&self) -> ColoredString {
        self.bold().cyan()
    }

    fn program(&self) -> ColoredString {
        self.bold().yellow()
    }

    fn success(&self) -> ColoredString {
        self.bold().green()
    }

    fn failure(&self) -> ColoredString {
        self.bold().red()
    }
}
