use colored::*;

pub trait Tend {
    fn thick(&self) -> ColoredString;
    fn job(&self) -> ColoredString;
    // fn program(&self) -> ColoredString;
    fn time_value(&self) -> ColoredString;
    fn success(&self) -> ColoredString;
    fn failure(&self) -> ColoredString;
}

impl Tend for str {
    fn thick(&self) -> ColoredString {
        self.bold()
    }

    fn job(&self) -> ColoredString {
        self.bold().cyan()
    }

    // fn program(&self) -> ColoredString { self.bold().yellow() }

    fn time_value(&self) -> ColoredString {
        self.bold().yellow()
    }

    fn success(&self) -> ColoredString {
        self.bold().green()
    }

    fn failure(&self) -> ColoredString {
        self.bold().red()
    }
}
