//! Abstract Syntax Tree
use options;

/// AST of an asm function
#[derive(Debug, Clone)]
pub struct Function {
    pub id: String,
    pub file: Option<File>,
    pub loc: Option<Loc>,
    pub statements: Vec<Statement>,
}

/// Statemets
#[derive(Debug, Clone)]
pub enum Statement {
    Label(Label),
    Directive(Directive),
    Instruction(Instruction),
    Comment(Comment),
}

/// Asm labels, e.g., LBB0:
#[derive(Debug, Clone)]
pub struct Label {
    pub id: String,
    rust_loc_off: Option<Loc>,
}

impl Label {
    pub fn new(s: &str, rust_loc_off: Option<Loc>) -> Option<Self> {
        if s.ends_with(":") {
            return Some(Self {
                id: s.split_at(s.len() - 1).0.to_string(),
                rust_loc_off,
            });
        }
        None
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        self.rust_loc_off
    }
    pub fn should_print(&self, _opts: &options::Options) -> bool {
        !self.id.starts_with("Lcfi") && !self.id.starts_with("Ltmp")
            && !self.id.starts_with("Lfunc_end")
    }
    pub fn format(&self, _opts: &options::Options) -> String {
        format!("  {}:", self.id)
    }
}

/// Asm directives, e.g, .static ...
#[derive(Debug, Clone)]
pub enum Directive {
    File(File),
    Loc(Loc),
    Generic(GenericDirective),
}

#[derive(PartialEq, Debug, Clone)]
pub struct File {
    pub path: String,
    pub index: usize,
}

impl File {
    pub fn new(s: &str) -> Option<Self> {
        if !s.starts_with(".file") {
            return None;
        }
        let path = s.split('"').nth(1).unwrap();
        let index = s.split_whitespace().nth(1).unwrap();
        Some(Self {
            path: path.to_string(),
            index: index.parse().unwrap(),
        })
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        None
    }
    pub fn format(&self, _opts: &options::Options) -> String {
        format!(".file {} \"{}\"", self.index, self.path)
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Loc {
    pub file_index: usize,
    pub file_line: usize,
    pub file_column: usize,
}

impl Loc {
    pub fn new(s: &str) -> Option<Self> {
        if !s.starts_with(".loc") {
            return None;
        }
        let mut it = s.split_whitespace();
        let file_index = it.nth(1).unwrap();
        let file_line = it.next().unwrap();
        let file_column = it.next().unwrap_or("0");
        Some(Self {
            file_index: file_index.parse().unwrap(),
            file_line: file_line.parse().unwrap(),
            file_column: file_column.parse().unwrap(),
        })
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        None
    }
    pub fn format(&self, _opts: &options::Options) -> String {
        format!(
            ".loc {} {} {}",
            self.file_index, self.file_line, self.file_column
        )
    }
}

#[derive(Clone, Debug)]
pub struct GenericDirective {
    string: String,
}

impl GenericDirective {
    pub fn new(s: &str) -> Option<Self> {
        if s.starts_with(".") {
            return Some(Self {
                string: s.to_string(),
            });
        }
        None
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        None
    }
    pub fn format(&self, _opts: &options::Options) -> String {
        format!("{}", self.string)
    }
}

impl Directive {
    pub fn new(s: &str) -> Option<Self> {
        if s.starts_with(".") {
            if let Some(file) = File::new(s) {
                return Some(Directive::File(file));
            }
            if let Some(loc) = Loc::new(s) {
                return Some(Directive::Loc(loc));
            }

            return Some(Directive::Generic(GenericDirective::new(s).unwrap()));
        }
        None
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        match *self {
            Directive::File(ref f) => f.rust_loc(),
            Directive::Loc(ref f) => f.rust_loc(),
            Directive::Generic(ref f) => f.rust_loc(),
        }
    }
    pub fn file(&self) -> Option<File> {
        match *self {
            Directive::File(ref f) => Some(f.clone()),
            _ => None,
        }
    }
    pub fn loc(&self) -> Option<Loc> {
        match *self {
            Directive::Loc(ref l) => Some(*l),
            _ => None,
        }
    }
    pub fn should_print(&self, opts: &options::Options) -> bool {
        opts.directives
    }
    pub fn format(&self, opts: &options::Options) -> String {
        match *self {
            Directive::File(ref f) => f.format(opts),
            Directive::Loc(ref f) => f.format(opts),
            Directive::Generic(ref f) => f.format(opts),
        }
    }
}

/// Asm comments, e.g, ;; this is a comment.
#[derive(Debug, Clone)]
pub struct Comment {
    string: String,
}

impl Comment {
    pub fn new(s: &str) -> Option<Self> {
        if s.starts_with(";") {
            return Some(Self {
                string: s.to_string(),
            });
        }
        None
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        None
    }
    pub fn should_print(&self, opts: &options::Options) -> bool {
        opts.comments
    }
    pub fn format(&self, _opts: &options::Options) -> String {
        format!("  {}", self.string)
    }
}

/// Asm instructions: everything else (not a Comment, Directive, or Label).
#[derive(Debug, Clone)]
pub struct Instruction {
    instr: String,
    args: Vec<String>,
    rust_loc_off: Option<Loc>,
}

impl Instruction {
    pub fn new(s: &str, rust_loc_off: Option<Loc>) -> Option<Self> {
        let mut iter = s.split_whitespace();
        let instr = iter.next().unwrap().to_string();
        let mut args = Vec::new();
        for arg in iter {
            args.push(arg.to_string());
        }
        if &instr == "call" {
            let demangled_function = ::demangle::demangle(&args[0]);
            args[0] = demangled_function;
        }
        return Some(Self {
            instr,
            args,
            rust_loc_off,
        });
    }
    pub fn rust_loc(&self) -> Option<Loc> {
        self.rust_loc_off
    }
    pub fn should_print(&self, _opts: &options::Options) -> bool {
        true
    }
    pub fn format(&self, opts: &options::Options) -> String {
        if opts.verbose {
            format!(
                "    {} {} | rloc: {:?}",
                self.instr,
                self.args.join(" "),
                self.rust_loc()
                    .as_ref()
                    .map(|v| (v.file_index, v.file_line))
            )
        } else {
            format!("    {} {}", self.instr, self.args.join(" "))
        }
    }
}

impl Statement {
    pub fn should_print(&self, opts: &options::Options) -> bool {
        match self {
            &Statement::Label(ref l) => l.should_print(opts),
            &Statement::Directive(ref l) => l.should_print(opts),
            &Statement::Instruction(ref l) => l.should_print(opts),
            &Statement::Comment(ref l) => l.should_print(opts),
        }
    }
    pub fn format(&self, opts: &options::Options) -> String {
        match self {
            &Statement::Label(ref l) => l.format(opts),
            &Statement::Directive(ref l) => l.format(opts),
            &Statement::Instruction(ref l) => l.format(opts),
            &Statement::Comment(ref l) => l.format(opts),
        }
    }
    pub fn rust_loc(&self, file: &File) -> Option<usize> {
        let loc = match self {
            &Statement::Label(ref l) => l.rust_loc(),
            &Statement::Directive(ref l) => l.rust_loc(),
            &Statement::Instruction(ref l) => l.rust_loc(),
            &Statement::Comment(ref l) => l.rust_loc(),
        };
        if loc.is_none() {
            return None;
        }
        let loc = loc.unwrap();
        if loc.file_index != file.index {
            return None;
        }
        Some(loc.file_line)
    }
}
