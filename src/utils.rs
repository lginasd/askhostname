const NEWLINE: &str =
if cfg!(windows) { "\r\n" }
else { "\n" };

pub trait AppendNewline {
    fn new_line(&mut self);
}
impl AppendNewline for String {
    fn new_line(&mut self) {
        self.push_str(NEWLINE);
    }
}
