//! Avaliador de expressões: `+ - * / ^`, parênteses, constantes (`pi`), funções
//! (`sin cos tan asin acos atan sqrt abs min max round floor ceil if`) e nomes de parâmetros.
//! Ângulos das funções trigonométricas em graus, como no restante do CAD.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ExprError {
    #[error("erro de sintaxe na posição {0}: {1}")]
    Syntax(usize, String),
    #[error("parâmetro desconhecido: {0}")]
    UnknownName(String),
    #[error("função desconhecida: {0}")]
    UnknownFunction(String),
    #[error("número de argumentos inválido em {0}")]
    Arity(String),
    #[error("dependência circular: {0}")]
    Cycle(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Num(f64),
    Name(String),
    Neg(Box<Ast>),
    Bin(char, Box<Ast>, Box<Ast>),
    Call(String, Vec<Ast>),
}

pub fn parse(src: &str) -> Result<Ast, ExprError> {
    let mut p = Parser { s: src.as_bytes(), i: 0 };
    let ast = p.expr()?;
    p.ws();
    if p.i < p.s.len() {
        return Err(ExprError::Syntax(p.i, "caractere inesperado".into()));
    }
    Ok(ast)
}

/// Nomes de parâmetros usados pela expressão (para o grafo de dependências).
pub fn names(ast: &Ast, out: &mut Vec<String>) {
    match ast {
        Ast::Num(_) => {}
        Ast::Name(n) => {
            if !CONSTS.iter().any(|(c, _)| c == n) && !out.contains(n) {
                out.push(n.clone())
            }
        }
        Ast::Neg(a) => names(a, out),
        Ast::Bin(_, a, b) => {
            names(a, out);
            names(b, out)
        }
        Ast::Call(_, args) => args.iter().for_each(|a| names(a, out)),
    }
}

const CONSTS: [(&str, f64); 2] = [("pi", std::f64::consts::PI), ("e", std::f64::consts::E)];

pub fn eval(ast: &Ast, vars: &HashMap<String, f64>) -> Result<f64, ExprError> {
    Ok(match ast {
        Ast::Num(v) => *v,
        Ast::Name(n) => match CONSTS.iter().find(|(c, _)| c == n) {
            Some((_, v)) => *v,
            None => *vars.get(n).ok_or_else(|| ExprError::UnknownName(n.clone()))?,
        },
        Ast::Neg(a) => -eval(a, vars)?,
        Ast::Bin(op, a, b) => {
            let (a, b) = (eval(a, vars)?, eval(b, vars)?);
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                '^' => a.powf(b),
                '<' => (a < b) as i32 as f64,
                '>' => (a > b) as i32 as f64,
                _ => unreachable!(),
            }
        }
        Ast::Call(f, args) => {
            let v: Vec<f64> = args.iter().map(|a| eval(a, vars)).collect::<Result<_, _>>()?;
            let one = |g: fn(f64) -> f64| -> Result<f64, ExprError> {
                if v.len() == 1 { Ok(g(v[0])) } else { Err(ExprError::Arity(f.clone())) }
            };
            match f.as_str() {
                "sin" => one(|x| x.to_radians().sin())?,
                "cos" => one(|x| x.to_radians().cos())?,
                "tan" => one(|x| x.to_radians().tan())?,
                "asin" => one(|x| x.asin().to_degrees())?,
                "acos" => one(|x| x.acos().to_degrees())?,
                "atan" => one(|x| x.atan().to_degrees())?,
                "sqrt" => one(f64::sqrt)?,
                "abs" => one(f64::abs)?,
                "round" => one(f64::round)?,
                "floor" => one(f64::floor)?,
                "ceil" => one(f64::ceil)?,
                "min" if !v.is_empty() => v.iter().cloned().fold(f64::INFINITY, f64::min),
                "max" if !v.is_empty() => v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                "if" if v.len() == 3 => if v[0] != 0.0 { v[1] } else { v[2] },
                "min" | "max" | "if" => return Err(ExprError::Arity(f.clone())),
                _ => return Err(ExprError::UnknownFunction(f.clone())),
            }
        }
    })
}

/// Avalia uma expressão isolada contra um conjunto de valores.
pub fn eval_str(src: &str, vars: &HashMap<String, f64>) -> Result<f64, ExprError> {
    eval(&parse(src)?, vars)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn err<T>(&self, m: &str) -> Result<T, ExprError> {
        Err(ExprError::Syntax(self.i, m.into()))
    }
    // comparação < >
    fn expr(&mut self) -> Result<Ast, ExprError> {
        let mut a = self.sum()?;
        while let Some(c @ (b'<' | b'>')) = self.peek() {
            self.i += 1;
            a = Ast::Bin(c as char, Box::new(a), Box::new(self.sum()?));
        }
        Ok(a)
    }
    fn sum(&mut self) -> Result<Ast, ExprError> {
        let mut a = self.term()?;
        while let Some(c @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            a = Ast::Bin(c as char, Box::new(a), Box::new(self.term()?));
        }
        Ok(a)
    }
    fn term(&mut self) -> Result<Ast, ExprError> {
        let mut a = self.unary()?;
        while let Some(c @ (b'*' | b'/')) = self.peek() {
            self.i += 1;
            a = Ast::Bin(c as char, Box::new(a), Box::new(self.unary()?));
        }
        Ok(a)
    }
    fn unary(&mut self) -> Result<Ast, ExprError> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Ok(Ast::Neg(Box::new(self.unary()?)))
            }
            Some(b'+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.pow(),
        }
    }
    fn pow(&mut self) -> Result<Ast, ExprError> {
        let base = self.atom()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            return Ok(Ast::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }
    fn atom(&mut self) -> Result<Ast, ExprError> {
        match self.peek() {
            Some(b'(') => {
                self.i += 1;
                let a = self.expr()?;
                if self.peek() != Some(b')') {
                    return self.err("esperado ')'");
                }
                self.i += 1;
                Ok(a)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let st = self.i;
                while self.i < self.s.len()
                    && (self.s[self.i].is_ascii_digit() || matches!(self.s[self.i], b'.' | b'e' | b'E'))
                {
                    // expoente com sinal: 1e-3
                    if matches!(self.s[self.i], b'e' | b'E') && matches!(self.s.get(self.i + 1), Some(b'-' | b'+')) {
                        self.i += 1;
                    }
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                t.parse().map(Ast::Num).or_else(|_| self.err("número inválido"))
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let st = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || matches!(self.s[self.i], b'_' | b'.')) {
                    self.i += 1;
                }
                let n = String::from_utf8(self.s[st..self.i].to_vec()).unwrap();
                if self.peek() == Some(b'(') {
                    self.i += 1;
                    let mut args = vec![];
                    if self.peek() != Some(b')') {
                        loop {
                            args.push(self.expr()?);
                            match self.peek() {
                                Some(b',' | b';') => self.i += 1,
                                _ => break,
                            }
                        }
                    }
                    if self.peek() != Some(b')') {
                        return self.err("esperado ')'");
                    }
                    self.i += 1;
                    Ok(Ast::Call(n.to_lowercase(), args))
                } else {
                    Ok(Ast::Name(n))
                }
            }
            _ => self.err("esperado número, nome ou '('"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedencia_e_funcoes() {
        let mut v = HashMap::new();
        v.insert("L".into(), 100.0);
        v.insert("folga".into(), 0.5);
        assert_eq!(eval_str("L/2 + folga", &v).unwrap(), 50.5);
        assert_eq!(eval_str("2^3^2", &v).unwrap(), 512.0);
        assert_eq!(eval_str("-2^2", &v).unwrap(), -4.0);
        assert!((eval_str("sin(30)", &v).unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(eval_str("if(L > 50, 10, 20)", &v).unwrap(), 10.0);
        assert_eq!(eval_str("max(1, L, 3)", &v).unwrap(), 100.0);
        assert_eq!(eval_str("1e-3*1000", &v).unwrap(), 1.0);
    }

    #[test]
    fn erros() {
        let v = HashMap::new();
        assert!(matches!(eval_str("x+1", &v), Err(ExprError::UnknownName(_))));
        assert!(matches!(eval_str("(1+2", &v), Err(ExprError::Syntax(..))));
        assert!(matches!(eval_str("foo(1)", &v), Err(ExprError::UnknownFunction(_))));
    }
}
