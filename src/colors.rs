use colored::{ColoredString, Colorize};

pub trait Tend {
    fn job(&self) -> ColoredString;
    fn failure(&self) -> ColoredString;
}

impl Tend for str {
    fn job(&self) -> ColoredString {
        self.bold().cyan()
    }

    fn failure(&self) -> ColoredString {
        self.bold().red()
    }
}
