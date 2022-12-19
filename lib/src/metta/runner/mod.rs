use crate::*;
use crate::common::shared::Shared;

use super::*;
use super::space::grounding::GroundingSpace;
use super::text::{Tokenizer, SExprParser};
use super::types::validate_atom;
use super::interpreter::interpret;

use std::path::PathBuf;
use std::collections::HashMap;

mod stdlib;

mod arithmetics;

const EXEC_SYMBOL : Atom = sym!("!");

pub struct Metta {
    space: Shared<GroundingSpace>,
    tokenizer: Shared<Tokenizer>,
    settings: Shared<HashMap<String, String>>,
}

enum Mode {
    ADD,
    INTERPRET,
}

impl Metta {
    pub fn new(space: Shared<GroundingSpace>, tokenizer: Shared<Tokenizer>) -> Self {
        Metta::from_space_cwd(space, tokenizer, PathBuf::from("."))
    }

    pub fn from_space_cwd(space: Shared<GroundingSpace>, tokenizer: Shared<Tokenizer>, cwd: PathBuf) -> Self {
        let settings = Shared::new(HashMap::new());
        let metta = Self{ space, tokenizer, settings };
        stdlib::register_common_tokens(&metta, cwd);
        metta
    }

    pub fn load_module(&self, name: &str) {
        let space = Shared::new(GroundingSpace::new());
        let tokenizer = self.tokenizer.clone();
        let settings = self.settings.clone();
        // We don't use Metta::[new|from_space_cwd] in order to use the right tokenizer
        // (and to avoid overriding it with Rust tokens)
        let runner = Self { space, tokenizer, settings };
        if name == "stdlib" {
            runner.run(&mut SExprParser::new(stdlib::metta_code())).expect("Cannot import stdlib code");
        }
        let space_atom = Atom::gnd(runner.space.clone());
        // self.tokenizer.borrow_mut().register_token(stdlib::regex(name), move |_| { space_atom.clone() });
        self.space.borrow_mut().add(space_atom);
    }

    pub fn space(&self) -> Shared<GroundingSpace> {
        self.space.clone()
    }

    pub fn tokenizer(&self) -> Shared<Tokenizer> {
        self.tokenizer.clone()
    }

    fn get_setting(&self, key: &str) -> Option<String> {
        self.settings.borrow().get(key.into()).cloned()
    }

    pub fn run(&self, parser: &mut SExprParser) -> Result<Vec<Vec<Atom>>, String> {
        let mut mode = Mode::ADD;
        let mut results: Vec<Vec<Atom>> = Vec::new();

        loop {
            let atom = parser.parse(&self.tokenizer.borrow());
            match atom {
                Some(atom) => {
                    if atom == EXEC_SYMBOL {
                        mode = Mode::INTERPRET;
                        continue;
                    }
                    match mode {
                        Mode::ADD => self.add_atom(atom),
                        Mode::INTERPRET => match self.evaluate_atom(atom) {
                            Err(msg) => return Err(msg),
                            Ok(result) => results.push(result),
                        },
                    }
                    mode = Mode::ADD;
                },
                None => break,
            }
        }
        Ok(results)
    }

    pub fn evaluate_atom(&self, atom: Atom) -> Result<Vec<Atom>, String> {
        match self.type_check(atom) {
            Err(atom) => Ok(vec![atom]),
            Ok(atom) => interpret(self.space.clone(), &atom),
        }
    }

    fn add_atom(&self, atom: Atom) {
        self.space.borrow_mut().add(atom);
    }

    fn type_check(&self, atom: Atom) -> Result<Atom, Atom> {
        let is_type_check_enabled = self.get_setting("type-check").map_or(false, |val| val == "auto");
        if  is_type_check_enabled && !validate_atom(&self.space.borrow(), &atom) {
            Err(Atom::expr([ERROR_SYMBOL, atom, BAD_TYPE_SYMBOL]))
        } else {
            Ok(atom)
        }
    }

}

pub fn new_metta_rust() -> Metta {
    let metta = Metta::new(Shared::new(GroundingSpace::new()),
        Shared::new(Tokenizer::new()));
    stdlib::register_rust_tokens(&metta);
    metta.load_module("stdlib");
    metta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space() {
        let program = "
            (= (And T T) T)
            (= (frog $x)
                (And (croaks $x)
                     (eat_flies $x)))
            (= (croaks Fritz) T)
            (= (eat_flies Fritz) T)
            (= (green $x) (frog $x))
            !(green Fritz)
        ";

        let metta = new_metta_rust();
        let result = metta.run(&mut SExprParser::new(program));
        assert_eq!(result, Ok(vec![vec![Atom::sym("T")]]));
    }
}
