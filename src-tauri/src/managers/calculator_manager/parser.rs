//! Recursive-descent arithmetic expression evaluator.
//!
//! Focused parser module used by the calculator manager.

struct Parser<'a> {
    src: &'a [char],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a [char]) -> Self {
        Self { src, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c == ' ') {
            self.pos += 1;
        }
    }

    /// expr = term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.consume();
                    left += self.parse_term()?;
                }
                Some('-') => {
                    self.consume();
                    left -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// term = power (('*' | '/') power)*
    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.consume();
                    let r = self.parse_power()?;
                    left *= r;
                }
                Some('/') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 {
                        return Err("division by zero".into());
                    }
                    left /= r;
                }
                Some('%') => {
                    self.consume();
                    let r = self.parse_power()?;
                    if r == 0.0 {
                        return Err("modulo by zero".into());
                    }
                    left %= r;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// power = unary ('^' unary)?
    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        self.skip_ws();
        if self.peek() == Some('^') {
            self.consume();
            let exp = self.parse_unary()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    /// unary = '-' unary | primary
    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        if self.peek() == Some('-') {
            self.consume();
            Ok(-self.parse_unary()?)
        } else if self.peek() == Some('+') {
            self.consume();
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    /// primary = number | constant | func '(' expr ')' | '(' expr ')'
    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.consume();
                let v = self.parse_expr()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.consume();
                } else {
                    return Err("expected ')'".into());
                }
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) if c.is_ascii_alphabetic() => self.parse_name(),
            other => Err(format!("unexpected {:?}", other)),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E')
        {
            self.consume();
        }
        let s: String = self.src[start..self.pos].iter().collect();
        s.parse::<f64>().map_err(|e| e.to_string())
    }

    fn parse_name(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.consume();
        }
        let name: String = self.src[start..self.pos].iter().collect();
        self.skip_ws();

        // Constants
        match name.as_str() {
            "pi" | "PI" => return Ok(std::f64::consts::PI),
            "e" | "E" if self.peek() != Some('(') => return Ok(std::f64::consts::E),
            "inf" => return Ok(f64::INFINITY),
            _ => {}
        }

        // Functions
        if self.peek() == Some('(') {
            self.consume();
            let arg = self.parse_expr()?;
            self.skip_ws();
            if self.peek() == Some(')') {
                self.consume();
            } else {
                return Err("expected ')'".into());
            }
            return match name.as_str() {
                "sqrt" => Ok(arg.sqrt()),
                "abs" => Ok(arg.abs()),
                "floor" => Ok(arg.floor()),
                "ceil" => Ok(arg.ceil()),
                "round" => Ok(arg.round()),
                "sin" => Ok(arg.sin()),
                "cos" => Ok(arg.cos()),
                "tan" => Ok(arg.tan()),
                "asin" => Ok(arg.asin()),
                "acos" => Ok(arg.acos()),
                "atan" => Ok(arg.atan()),
                "ln" => Ok(arg.ln()),
                "log" | "log10" => Ok(arg.log10()),
                "log2" => Ok(arg.log2()),
                "exp" => Ok(arg.exp()),
                _ => Err(format!("unknown function '{}'", name)),
            };
        }

        Err(format!("unknown name '{}'", name))
    }
}

pub fn eval_expr(expr: &str) -> Result<f64, String> {
    let chars: Vec<char> = expr.chars().collect();
    let mut parser = Parser::new(&chars);
    let result = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos < parser.src.len() {
        return Err(format!("unexpected input at position {}", parser.pos));
    }
    if result.is_nan() {
        return Err("result is NaN".into());
    }
    Ok(result)
}
