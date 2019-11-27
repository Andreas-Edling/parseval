

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
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
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
                children: vec![],
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
                children: vec![],
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
                children: vec![],
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


fn parent_element<'a>() -> impl Parser<'a, Element> {
    and_then(
        opening_element(),
        |elem1|{
            map(
                left(
                    zero_or_more(element()), 
                    closing_element(elem1.name.clone())
                ),
                move |children| {
                    let mut elem1 = elem1.clone();
                    elem1.children = children;
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
            children: vec![
                Element {
                    name: "semi-bottom".to_string(),
                    attributes: vec![("label".to_string(), "Bottom".to_string())],
                    children: vec![],
                },
                Element {
                    name: "middle".to_string(),
                    attributes: vec![],
                    children: vec![Element {
                        name: "bottom".to_string(),
                        attributes: vec![("label".to_string(), "Another bottom".to_string())],
                        children: vec![],
                    }],
                },
            ],
        };
        assert_eq!(Ok(("", parsed_doc)), element().parse(doc));
    }

}