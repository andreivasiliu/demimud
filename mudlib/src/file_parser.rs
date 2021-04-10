pub(crate) struct FileParser<'a> {
    remaining: &'a str,
    all_text: &'a str,
    file_name: &'a str,
}

impl<'a> FileParser<'a> {
    pub fn new(text: &'a str, file_name: &'a str) -> Self {
        Self {
            remaining: text,
            all_text: text,
            file_name,
        }
    }

    pub fn panic_on_line(&self, message: &str) -> ! {
        let bytes_read = self.all_text.len() - self.remaining.len();
        let processed_slice = &self.all_text[0..bytes_read];
        let lines = processed_slice.chars().filter(|c| *c == '\n').count();
        let columns = processed_slice
            .chars()
            .rev()
            .take_while(|c| *c != '\n')
            .count();
        let last_line = &processed_slice[processed_slice.len() - columns..];

        panic!(
            "In file {}, line {}, column {}: {}\nLast line: {:?}\n",
            self.file_name,
            lines + 1,
            columns,
            message,
            last_line
        );
    }

    pub fn read_section(&mut self) -> &'a str {
        let start = self.remaining.find(|c: char| !c.is_whitespace()).unwrap();
        let end = self.remaining[start..]
            .find(|c: char| c.is_whitespace())
            .unwrap();

        if &self.remaining[start..start + 1] != "#" {
            self.panic_on_line(&format!(
                "Expected '#', got '{}'",
                &self.remaining[start..start + 1]
            ))
        }

        let section = &self.remaining[start + 1..start + end];
        self.remaining = &self.remaining[start + end..];
        self.skip_one_newline();
        section
    }

    pub fn read_word(&mut self) -> &'a str {
        let start = self
            .remaining
            .find(|c: char| !c.is_ascii_whitespace())
            .unwrap();
        let end = self.remaining[start..]
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap();

        let section = &self.remaining[start..start + end];
        self.remaining = &self.remaining[start + end..];
        section
    }

    pub fn skip_one_newline(&mut self) {
        if self.remaining.is_empty() {
            // Nothing to skip
        } else if self.remaining.starts_with('\r') {
            self.remaining = &self.remaining[1..];
            if self.remaining.starts_with('\n') {
                self.remaining = &self.remaining[1..];
            }
        } else if self.remaining.starts_with('\n') {
            self.remaining = &self.remaining[1..];
        } else {
            self.panic_on_line("No newline found to skip");
        }
    }

    pub fn skip_one_space(&mut self) {
        if self.remaining.chars().next() != Some(' ') {
            self.panic_on_line(&format!("Expected ' ', got '{}'", &self.remaining[..1]))
        }
        self.remaining = &self.remaining[1..];
    }

    pub fn skip_all_space(&mut self) {
        let start = self
            .remaining
            .find(|c: char| !c.is_ascii_whitespace())
            .unwrap_or(0);
        self.remaining = &self.remaining[start..];
    }

    pub fn read_until_newline(&mut self) -> &'a str {
        let end = self.remaining.find('\n').unwrap();

        let section = &self.remaining[..end];
        self.remaining = &self.remaining[end..];
        self.skip_one_newline();

        if section.ends_with('\r') {
            &section[..section.len() - 1]
        } else {
            section
        }
    }

    pub fn read_until_tilde(&mut self) -> &'a str {
        let end = self.remaining.find('~').unwrap();

        let section = &self.remaining[..end];
        self.remaining = &self.remaining[end + 1..];
        self.skip_one_newline();
        section
    }
}
