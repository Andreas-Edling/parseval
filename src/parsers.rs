
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsingError {
    FailedWith(String),
}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsingError::FailedWith(s) => write!(f,"ParsingError failed with remaining data to parse: {}",s),
        }
    }
}

impl std::error::Error for ParsingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParsingError::FailedWith(_) => None,
        }
    }
}

pub type ParseResult<'a, T> = Result<(&'a str, T), ParsingError>;

pub trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;
}

impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>,
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

pub struct BoxedParser<'a, Output> {
    boxed: Box<dyn Parser<'a, Output> + 'a>,

}

impl<'a, Output> BoxedParser<'a, Output> {
    pub fn new<P>(parser: P) -> Self 
    where P: Parser<'a, Output> + 'a
    {
        BoxedParser{ boxed: Box::new(parser) }
    }
}

impl<'a, Output> Parser<'a, Output> for BoxedParser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self.boxed.parse(input)
    }
}


// -- combinators ---------------------------------------------------------------

pub fn match_literal<'a>(expected: &'static str) -> impl Parser<'a, ()> {
    move |input: &'a str| {
        match input.get(0..expected.len()) {
            Some(next) if next == expected => Ok((&input[expected.len()..], ())),
            _ => Err(ParsingError::FailedWith(input.to_string())),
        }
    }
}

pub fn pair<'a, P1,P2,R1,R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1,R2)> 
    where 
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| {
        parser1.parse(input).and_then( |(input2, result1)| {
            parser2.parse(input2)
                .map(|(input2, result2)|{
                    (input2, (result1, result2)) 
                })
        })
    }
}

pub fn map<'a, P, R1, F, R2>(parser: P, f: F) -> BoxedParser<'a, R2>
where 
    P: Parser<'a, R1> + 'a,
    F: Fn(R1)->R2 + 'a,
    R1: 'a,
    R2: 'a,
{
    BoxedParser::new(
        move |input| {
            parser.parse(input).map( |(input2, r1)| (input2, f(r1)))
        }
    )
}

pub fn left<'a, P1,P2,R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R1> 
    where 
    P1: Parser<'a, R1> + 'a,
    P2: Parser<'a, R2> + 'a,
    R1: 'a,
    R2: 'a,
{
    map(pair(parser1, parser2), |(left, _right)| left)
}

pub fn right<'a, P1,P2,R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R2> 
    where 
    P1: Parser<'a, R1> + 'a,
    P2: Parser<'a, R2> + 'a,
    R1: 'a,
    R2: 'a,
{
    map(pair(parser1, parser2), |(_left, right)| right)
}

pub fn pred<'a, P, R, F>(parser: P, predicate: F) -> impl Parser<'a, R>
where
    P: Parser<'a, R>,
    F: Fn(&R)->bool
{
    move |input| {
        match parser.parse(input) {
            Ok((remaining_input, value)) if predicate(&value) => Ok((remaining_input, value)),
            _ => Err(ParsingError::FailedWith(input.to_string()))
        }
    }
}

pub fn zero_or_more<'a, P, R>(parser: P) -> impl Parser<'a, Vec<R>> 
where
    P: Parser<'a, R>,
{
    move |mut input| {
        let mut result_vec = Vec::new();
        while let Ok((remaining_input, value)) = parser.parse(input) {
            input = remaining_input;
            result_vec.push(value);
        }
        Ok((input, result_vec))
    }
}

pub fn one_or_more<'a, P, R>(parser: P) -> impl Parser<'a, Vec<R>>
where
    P: Parser<'a, R>,
{
    move |mut input| {
        let mut result_vec = Vec::new();
        if let Ok((remaining_input, value)) = parser.parse(input) {
            input = remaining_input;
            result_vec.push(value);
        } else {
            return Err(ParsingError::FailedWith(input.to_string()));
        }

        while let Ok((remaining_input, value)) = parser.parse(input) {
            input = remaining_input;
            result_vec.push(value);
        }
        Ok((input, result_vec))
    }
}

pub fn whitespace_char<'a>() -> impl Parser<'a, char> {
    pred(any_char, |c| c.is_whitespace())
}

pub fn whitespace0<'a>() -> impl Parser<'a, Vec<char>> {
    zero_or_more(whitespace_char())
}

pub fn whitespace1<'a>() -> impl Parser<'a, Vec<char>> {
    one_or_more(whitespace_char())
}

pub fn trim<'a, P, R>(parser: P) -> impl Parser<'a, R> 
where
    P: Parser<'a, R> + 'a,
    R: 'a,
{
    right(whitespace0(), left(parser, whitespace0()))
}

pub fn string_in_quotes<'a>() -> impl Parser<'a, String> {
    let between_dquotes = zero_or_more(pred(any_char, |c| *c != '\"'));
    let string_with_quotes =
        right( 
            match_literal("\""),
            left(
                between_dquotes,
                match_literal("\"")
            )
        );

    map(
        string_with_quotes,    
        |characters| characters.into_iter().collect()
    )
}

pub fn either<'a, P1, P2, R>(parser1: P1, parser2: P2) -> BoxedParser<'a, R>
where
    P1: Parser<'a, R> + 'a,
    P2: Parser<'a, R> + 'a,
    R: 'a,
{
    BoxedParser::new(
        move |input| match parser1.parse(input) {
            Ok(x) => Ok(x),
            Err(_) => parser2.parse(input),
        }
    )
}

pub fn and_then<'a, P, R1, F, R2, NextP>(parser: P, f: F) -> BoxedParser<'a, R2>
where
    P: Parser<'a, R1> + 'a,
    NextP: Parser<'a, R2> + 'a,
    F: Fn(R1) -> NextP +'a,
    R1: 'a,
    R2: 'a,
{
    BoxedParser::new(
        move |input| {
            match parser.parse(input) {
                Ok((remaining_input, value)) => f(value).parse(remaining_input),
                Err(e) => Err(e)
            }
        }
    )
}

// -- data parsers --------------------------------------------------------------


pub fn array_f32<'a>() -> impl Parser<'a, Vec<f32>> {
    
    let number =
        one_or_more(
            pred(
                any_char,
                |c| c.is_ascii_digit() || *c == '-' || *c == '.' || *c == 'e' || *c == 'E'
            )
        );

    let number = map(
        left(
            number, 
            whitespace0()
        ),
        |chars| {
            let string: String = chars.into_iter().collect();
            string.parse::<f32>().unwrap()
        }
    );

    zero_or_more(number)
}

pub fn array_u32<'a>() -> impl Parser<'a, Vec<u32>> {
    let number_str = 
    one_or_more(
        pred(
            any_char,
            |c| c.is_ascii_digit() || *c == '-'
        )
    );

    let number = map(
        left(number_str, whitespace0()),
        |chars| {
            let string: String = chars.into_iter().collect();
            string.parse::<u32>().unwrap()
        }
    );

    zero_or_more(number)
}

// -- 'raw' parsers -------------------------------------------------------------

pub fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(ParsingError::FailedWith(input.to_string())),
    }
}
