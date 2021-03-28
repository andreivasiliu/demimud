pub(crate) struct FileParser<'a>(&'a str, &'a str);

impl<'a> FileParser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self(text, text)
    }

    pub fn panic_on_line(&self, message: &str) -> ! {
        let bytes_read = self.1.len() - self.0.len();
        let processed_slice = &self.1[0..bytes_read];
        let lines = processed_slice.chars().filter(|c| *c == '\n').count();
        let columns = processed_slice
            .chars()
            .rev()
            .take_while(|c| *c != '\n')
            .count();
        let last_line = &processed_slice[processed_slice.len() - columns..];

        panic!(
            "On line {}, column {}: {}\nLast line:\n{}\n",
            lines + 1,
            columns,
            message,
            last_line
        );
    }

    pub fn read_section(&mut self) -> &'a str {
        let start = self.0.find(|c: char| !c.is_whitespace()).unwrap();
        let end = self.0[start..].find(|c: char| c.is_whitespace()).unwrap();

        if &self.0[start..start + 1] != "#" {
            self.panic_on_line(&format!(
                "Expected '#', got '{}'",
                &self.0[start..start + 1]
            ))
        }

        let mut section = &self.0[start + 1..start + end];
        if section.chars().last() == Some('\r') {
            section = &section[..section.len() - 1]
        }
        self.0 = &self.0[start + end + 1..];
        self.skip_one_newline();
        section
    }

    pub fn read_word(&mut self) -> &'a str {
        let start = self.0.find(|c: char| !c.is_ascii_whitespace()).unwrap();
        let end = self.0[start..]
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end..];
        section
    }

    pub fn skip_one_newline(&mut self) {
        if self.0.is_empty() {
            return;
        } else if self.0.chars().next() == Some('\r') {
            self.0 = &self.0[1..];
            if self.0.chars().next() == Some('\n') {
                self.0 = &self.0[1..];
            }
        } else if self.0.chars().next() == Some('\n') {
            self.0 = &self.0[1..];
        } else {
            self.panic_on_line("No newline found to skip");
        }
    }

    pub fn skip_one_space(&mut self) {
        if &self.0[..1] != " " {
            self.panic_on_line(&format!("Expected ' ', got '{}'", &self.0[..1]))
        }
        self.0 = &self.0[1..];
    }

    pub fn skip_all_space(&mut self) {
        let start = self.0.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0);
        self.0 = &self.0[start..];
    }

    pub fn read_until_newline(&mut self) -> &'a str {
        let start = 0;
        let end = self.0[start..]
            .find(|c: char| c == '\n' || c == '\r')
            .unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end..];
        self.skip_one_newline();
        section
    }

    pub fn read_until_tilde(&mut self) -> &'a str {
        let start = 0;
        let end = self.0[start..].find(|c: char| c == '~').unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end + 1 + 1..];
        self.skip_one_newline();
        section
    }
}
