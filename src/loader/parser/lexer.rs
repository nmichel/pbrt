#[derive(Debug, PartialEq)]
pub enum Token {
    Identifier(String),
    KWAABox,
    KWCheckerboard,
    KWCSG,
    KWElem,
    KWIntersection,
    KWScene,
    KWSphere,
    KWCompound,
    KWCylinder,
    BraceOpen,
    BraceClose,
    Comment,
    SquareOpen,
    SquareClose,
    ParenOpen,
    ParenClose,
    Number(f64),
    KWColor,
    KWLambertian,
    KWObject,
    KWPlane,
    KWRectangle,
    KWSimple,
    KWTransform,
    KWTransformed,
    KWTranslate,
    KWUnion,
    KWSubtraction,
    KWRotateX,
    KWRotateY,
    KWRotateZ,
}

impl<'a> From<&'a str> for Token {
    fn from(other: &'a str) -> Token {
        match other {
            "aabox" => Token::KWAABox,
            "checkerboard" => Token::KWCheckerboard,
            "color" => Token::KWColor,
            "csg" => Token::KWCSG,
            "compound" => Token::KWCompound,
            "cylinder" => Token::KWCylinder,
            "elem" => Token::KWElem,
            "intersection" => Token::KWIntersection,
            "lambertian" => Token::KWLambertian,
            "object" => Token::KWObject,
            "plane" => Token::KWPlane,
            "rectangle" => Token::KWRectangle,
            "rotate_x" => Token::KWRotateX,
            "rotate_y" => Token::KWRotateY,
            "rotate_z" => Token::KWRotateZ,
            "scene" => Token::KWScene,
            "simple" => Token::KWSimple,
            "sphere" => Token::KWSphere,
            "substraction" => Token::KWSubtraction,
            "transform" => Token::KWTransform,
            "transformed" => Token::KWTransformed,
            "translate" => Token::KWTranslate,
            "union" => Token::KWUnion,
            _ => Token::Identifier(other.to_string()),
        }
    }
}

impl From<f64> for Token {
    fn from(other: f64) -> Token {
        Token::Number(other)
    }
}

pub struct Tokenizer<'a> {
    input: &'a str,
    byte_pos: usize,
    i: std::iter::Peekable<std::str::Chars<'a>>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Tokenizer {
            input: input,
            byte_pos: 0,
            i: input.chars().peekable(),
            pos: 0,
            line: 0,
            col: 0,
        }
    }

    /// Consumes non significant characters
    /// then returns the next token in the input string.
    ///
    /// # Arguments
    /// * `self` - The tokenizer
    /// ```
    pub fn next_token(self: &mut Self) -> Option<Token> {
        self.skip_whitespace();
        self.parse_next_token()
    }

    fn parse_next_token(self: &mut Self) -> Option<Token> {
        if let Some(c) = self.peek() {
            if let Some(token) = Self::parse_special_char(c) {
                self.next();

                if token == Token::Comment {
                    self.skip_comment();
                    self.next_token()
                }
                else {
                    Some(token)
                }
            }
            else {
                match c {
                    '0'..='9' | '-' | '+' => self.tokenize_number(),
                    m if m.is_alphabetic() => self.tokenize_identifier(),
                    _ => None,
                }
            }
        }
        else {
            None
        }
    }

    fn parse_special_char(c: &char) -> Option<Token> {
        match c {
            '{' => Some(Token::BraceOpen),
            '}' => Some(Token::BraceClose),
            '[' => Some(Token::SquareOpen),
            ']' => Some(Token::SquareClose),
            '(' => Some(Token::ParenOpen),
            ')' => Some(Token::ParenClose),
            '#' => Some(Token::Comment),
            _ => None,
        }
    }

    fn tokenize_number(self: &mut Self) -> Option<Token> {
        enum State {
            Sign,
            FirstDigit,
            Integer,
            Dot,
            Decimal,
        }

        let mut state: State = State::Sign;

        let digit_substr = self.take_while(|c| {
            match state {
                State::Sign => {
                    if c == &'-' || c == &'+' {
                        state = State::FirstDigit;
                        true
                    }
                    else if c.is_numeric() {
                        state = State::Integer;
                        true
                    }
                    else {
                        false
                    }
                }
                State::FirstDigit => {
                    if c.is_numeric() {
                        state = State::Integer;
                        true
                    }
                    else {
                        false
                    }
                }
                State::Integer => {
                    if c.is_numeric() {
                        true
                    }
                    else if c == &'.' {
                        state = State::Dot;
                        true
                    }
                    else {
                        false
                    }
                }
                State::Dot => {
                    if c.is_numeric() {
                        state = State::Decimal;
                        true
                    }
                    else {
                        false
                    }
                }
                State::Decimal => {
                    if c.is_numeric() {
                        true
                    }
                    else {
                        false
                    }
                }
            }
        });

        if let Some(str) = digit_substr {
            Some(Token::from(str.parse::<f64>().unwrap()))
        }
        else {
            None
        }
    }

    fn tokenize_identifier(self: &mut Self) -> Option<Token> {
        if let Some(str) = self.take_while(|c| c.is_alphanumeric() || c == &'_') {
            Some(Token::from(str))
        }
        else {
            None
        }
    }

    fn skip_whitespace(self: &mut Self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                if c == &'\n' {
                    self.line += 1;
                    self.col = 0;
                }
                else {
                    self.col += 1;
                }
                self.next();
            }
            else {
                break;
            }
        }
    }

    fn skip_comment(self: &mut Self) {
        self.take_while(|c| c != &'\n');
    }

    fn take_while<F>(self: &mut Self, mut pred: F) -> Option<&str>
    where
        F: FnMut(&char) -> bool,
    {
        let mut len: usize = 0;
        while let Some(c) = self.peek() {
            if pred(c) {
                len += c.len_utf8();
                self.next();
            }
            else {
                break;
            }
        }

        if len > 0 {
            Some(&self.input[self.byte_pos - len..self.byte_pos])
        }
        else {
            None
        }
    }

    fn peek(self: &mut Self) -> Option<&char> {
        self.i.peek()
    }

    fn next(self: &mut Self) {
        if let Some(c) = self.i.next() {
            self.byte_pos += c.len_utf8();
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let input = "hello world";
        let tokenizer = Tokenizer::new(input);
        assert_eq!(tokenizer.input, input);
        assert_eq!(tokenizer.byte_pos, 0);
        assert_eq!(tokenizer.pos, 0);
        assert_eq!(tokenizer.line, 0);
        assert_eq!(tokenizer.col, 0);
    }

    // test next_token
    #[test]
    fn test_valid_token() {
        assert_eq!(Tokenizer::new("foo").next_token().unwrap(), Token::Identifier("foo".to_string()));
        assert_eq!(
            Tokenizer::new("bar_neh_").next_token().unwrap(),
            Token::Identifier("bar_neh_".to_string())
        );
        assert_eq!(Tokenizer::new("10").next_token().unwrap(), Token::Number(10.0));
        assert_eq!(Tokenizer::new("10.").next_token().unwrap(), Token::Number(10.0));
        assert_eq!(Tokenizer::new("10.0").next_token().unwrap(), Token::Number(10.0));
        assert_eq!(Tokenizer::new("1234.56").next_token().unwrap(), Token::Number(1234.56));
        assert_eq!(Tokenizer::new("+1234.56").next_token().unwrap(), Token::Number(1234.56));
        assert_eq!(Tokenizer::new("-1234.56").next_token().unwrap(), Token::Number(-1234.56));
    }

    #[test]
    fn test_keywords() {
        assert_eq!(Tokenizer::new("aabox").next_token().unwrap(), Token::KWAABox);
        assert_eq!(Tokenizer::new("cylinder").next_token().unwrap(), Token::KWCylinder);
        assert_eq!(Tokenizer::new("sphere").next_token().unwrap(), Token::KWSphere);
    }

    #[test]
    fn test_invalid_token() {
        assert_eq!(Tokenizer::new("_foo").next_token(), None);
        assert_eq!(Tokenizer::new(".1").next_token(), None);
    }

    #[test]
    fn test_token_list() {
        let input = "     

    hello world 
        bla_bla
          aabox 1.6 2.3 3.4
          cylinder 1.6
            sphere -2.4
      Löf ";
        let mut tokenizer = Tokenizer::new(input);
        let mut tokens = Vec::new();
        while let Some(token) = tokenizer.next_token() {
            tokens.push(token);
        }
        assert_eq!(tokens.len(), 12);
        assert_eq!(tokens, vec![
            Token::Identifier("hello".to_string()),
            Token::Identifier("world".to_string()),
            Token::Identifier("bla_bla".to_string()),
            Token::KWAABox,
            Token::Number(1.6),
            Token::Number(2.3),
            Token::Number(3.4),
            Token::KWCylinder,
            Token::Number(1.6),
            Token::KWSphere,
            Token::Number(-2.4),
            Token::Identifier("Löf".to_string())
        ]);
    }

    #[test]
    fn test_comment() {
        let input = "     
        # sphere comment
    sphere # end of line comment
      Löf 
      # Comment alone";

        let mut tokenizer = Tokenizer::new(input);
        let mut tokens = Vec::new();
        while let Some(token) = tokenizer.next_token() {
            tokens.push(token);
        }
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens, vec![Token::KWSphere, Token::Identifier("Löf".to_string())]);
    }
}
