

use crate::parsers::{
    Parser,
    ParseResult,
    pair,
    left,
    right,
    match_literal,
    string_in_quotes,
    zero_or_more,
    whitespace1,
    trim,
    map,
    either,
    pred,
    and_then,
    any_char,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataOrElements {
    Data(String),
    Elements(Vec<Element>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub data_or_elements: DataOrElements,
}

pub fn element<'a>() -> impl Parser<'a, Element> {
    trim(either(single_element(), parent_element()))
}



fn identifier(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(input),
    }

    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '-' || next == ':' {
            matched.push(next);
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

fn attribute_pair<'a>() -> impl Parser<'a, (String, String)> {
    pair(
        identifier, 
        right(match_literal("="), string_in_quotes())
    )
}

fn attributes<'a>() -> impl Parser<'a, Vec<(String, String)>> {
    zero_or_more(
        right(
            whitespace1(), 
            attribute_pair()
        )
    )
}

fn element_start<'a>() -> impl Parser<'a, (String, Vec<(String, String)>)> {
    right(
        match_literal("<"), 
        pair(identifier, attributes())
    )
}

pub fn single_element<'a>() -> impl Parser<'a, Element> {
    map(
        left(element_start(), match_literal("/>")),
        |(name, attributes)| {
            Element {
                name,
                attributes,
                data_or_elements: DataOrElements::Elements(vec![]),
            }
        }
    )
}

pub fn xml_definition_element<'a>() -> impl Parser<'a, Element> {
    let start = right(
        match_literal("<?"), 
        pair(identifier, attributes())
    );

   map(
        trim(left(start, match_literal("?>"))),
        |(name, attributes)| {
            Element {
                name,
                attributes,
                data_or_elements: DataOrElements::Elements(vec![]),
            }
        }
    )
}

pub fn opening_element<'a>() -> impl Parser<'a, Element> {
    map(
        trim(left(element_start(), match_literal(">"))),
        |(name, attributes)| {
            Element {
                name,
                attributes,
                data_or_elements: DataOrElements::Elements(vec![]),
            }
        }
    )
}

pub fn closing_element<'a>(expected_name: String) -> impl Parser<'a, String> {
    pred(
        trim(right(
            match_literal("</"), 
            left(
                identifier, 
                match_literal(">")
            )
        )),
        move |name| name == &expected_name
    )
}


fn data<'a>() -> impl Parser<'a, String> {
    map( 
        zero_or_more(pred(any_char, |c| *c != '<')),
        |characters| characters.into_iter().collect()
    )
}

fn data_or_elements<'a>() -> impl Parser<'a, DataOrElements> {
    either(
        map(
            zero_or_more(element()),
            |elements| {
                DataOrElements::Elements(elements)
            }
        ),
        map(
            data(),
            |data| {
                DataOrElements::Data(data)
            }
        )
    )
}

fn parent_element<'a>() -> impl Parser<'a, Element> {
    and_then(
        opening_element(),
        |elem1|{
            map(
                left(
                    data_or_elements(), //zero_or_more(element()), 
                    closing_element(elem1.name.clone())
                ),
                move |data_or_elements| {
                    let mut elem1 = elem1.clone();
                    elem1.data_or_elements = data_or_elements;
                    elem1
                }
            )
        }
    )
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_parser() {
        assert_eq!(
            Ok((
                "",
                vec![
                    ("one".to_string(), "1".to_string()),
                    ("two".to_string(), "2".to_string())
                ]
            )),
            attributes().parse(" one=\"1\" two=\"2\"")
        );
    }
    
    #[test]
    fn xml_parser() {
        let doc = r#"
            <top label="Top">
                <semi-bottom label="Bottom"/>
                <middle>
                    <bottom label="Another bottom"/>
                </middle>
            </top>"#;
        let parsed_doc = Element {
            name: "top".to_string(),
            attributes: vec![("label".to_string(), "Top".to_string())],
            data_or_elements: DataOrElements::Elements(vec![
                Element {
                    name: "semi-bottom".to_string(),
                    attributes: vec![("label".to_string(), "Bottom".to_string())],
                    data_or_elements: DataOrElements::Elements(vec![]),
                },
                Element {
                    name: "middle".to_string(),
                    attributes: vec![],
                    data_or_elements: DataOrElements::Elements(vec![
                        Element {
                            name: "bottom".to_string(),
                            attributes: vec![("label".to_string(), "Another bottom".to_string())],
                            data_or_elements: DataOrElements::Elements(vec![]),
                        }
                    ]),
                },
            ],)
        };
        assert_eq!(Ok(("", parsed_doc)), element().parse(doc));
    }

}