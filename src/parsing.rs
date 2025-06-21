use anyhow::Result;
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct PrologParser;
pub fn parse(input: &str) -> Result<AbstractTerm> {
    let pairs = PrologParser::parse(Rule::term, input)?;
    let pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No term found"))?;
    let term = parse_term(pair)?;
    Ok(term)
}

fn parse_term(pair: Pair<'_, Rule>) -> Result<AbstractTerm> {
    let pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No term found"))?;

    match pair.as_rule() {
        Rule::term_variable => {
            let variable = pair.as_str().to_string();
            Ok(AbstractTerm::Variable(variable))
        }
        Rule::term_constant => {
            let constant = pair.as_str().to_string();
            Ok(AbstractTerm::Constant(constant))
        }
        Rule::term_structure => {
            let mut inner_pairs = pair.into_inner();
            let functor = inner_pairs
                .next()
                .ok_or_else(|| anyhow::anyhow!("No functor found"))?
                .as_str()
                .to_string();
            let args: Result<Vec<AbstractTerm>> = inner_pairs.map(|p| parse_term(p)).collect();
            Ok(AbstractTerm::Structure(functor, args?))
        }
        _ => Err(anyhow::anyhow!("Unexpected rule: {:?}", pair.as_rule())),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbstractTerm {
    Variable(String),
    Constant(String),
    Structure(String, Vec<AbstractTerm>),
}

impl AbstractTerm {
    pub fn arity(&self) -> usize {
        match self {
            AbstractTerm::Variable(_) => 0,
            AbstractTerm::Constant(_) => 0,
            AbstractTerm::Structure(_, args) => args.len(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            AbstractTerm::Variable(name) => name,
            AbstractTerm::Constant(name) => name,
            AbstractTerm::Structure(name, _) => name,
        }
    }
}
