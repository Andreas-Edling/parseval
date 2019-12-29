

use crate::parsers::{
    Parser,
    ParseResult,
    ParsingError,
    pair,
    left,
    right,
    match_literal,
    string_in_quotes,
    zero_or_more,
    one_or_more,
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

impl Element {
    pub fn get_attrib_value<'a>(&'a self, expected_attrib_name: &str) -> Result<&'a str, ElementError> {
        for (attrib_name, attrib_val) in &self.attributes {
            if expected_attrib_name == *attrib_name {
                return Ok(attrib_val);
            }
        }
        Err(ElementError::CantGetAttribValue(expected_attrib_name.to_string()))
    }

    pub fn get_child_by_attrib(&self, attrib: (&str, String)) -> Result<&Element, ElementError> {
        if let DataOrElements::Elements(children) = &self.data_or_elements {
            for element in children {
                for (attrib_name, attrib_val) in &element.attributes {
                    if attrib.0 == *attrib_name && attrib.1 == *attrib_val {
                        return Ok(element);
                    }
                }
            }
        }
        Err(ElementError::CantGetChildByAttrib((attrib.0.to_string(), attrib.1)))
    }

    pub fn get_child_by_name(&self, name: &str) -> Result<&Element, ElementError> {
        if let DataOrElements::Elements(children) = &self.data_or_elements {
            for child in children {
                if child.name == name {
                    return Ok(child);
                }
            }
        }
        Err(ElementError::CantGetChildByName(name.to_string()))
    }

    pub fn get_as_data(&self) -> Result<&str, ElementError> {
        match &self.data_or_elements {
            DataOrElements::Data(data) => Ok(data), 
            DataOrElements::Elements(_) => Err(ElementError::CantGetAsData),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ElementError {
    CantGetAttribValue(String),
    CantGetChildByAttrib((String, String)),
    CantGetChildByName(String),
    CantGetAsData,
}

impl std::fmt::Display for ElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementError::CantGetAttribValue(s) => write!(f,"ElementError::CantGetAttribValue: {}",s),
            ElementError::CantGetChildByAttrib((k,v)) => write!(f,"ElementError::CantGetChildByAttrib: ({},{})", k, v),
            ElementError::CantGetChildByName(s) => write!(f,"ElementError::CantGetChildByName: {}",s),
            ElementError::CantGetAsData => write!(f, "ElementError::CantGetAsData (child is element, not data)"),
        }
    }
}

impl std::error::Error for ElementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ElementError::CantGetAttribValue(_) => None,
            ElementError::CantGetChildByAttrib((_,_)) => None,
            ElementError::CantGetChildByName(_) => None,
            ElementError::CantGetAsData => None,
        }
    }
}

// -- parsers --------------------------------------------------------------------------

pub fn element<'a>() -> impl Parser<'a, Element> {
    trim(either(single_element(), parent_element()))
}

pub fn element_with_name<'a>(expected_name: String) -> impl Parser<'a, Element> {
    pred(
        trim(either(single_element(), parent_element())),
        move |elem| elem.name == expected_name
    )
}

fn identifier(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(ParsingError::FailedWith(input.to_string())),
    }

    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '-' || next == '_' || next == ':' {
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
            one_or_more(element()),
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
                    data_or_elements(),
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
    fn test_data_or_elements_with_data() {
        let doc = r#"foo</elem>"#;
        let res = data_or_elements().parse(doc);
        assert_eq!(Ok(("</elem>", DataOrElements::Data("foo".to_string()))), res);
    }

    #[test]
    fn test_data_or_elements_with_element() {
        let doc = r#"<foo/></elem>"#;
        let res = data_or_elements().parse(doc);
        let expected = DataOrElements::Elements(vec![
            Element{
                name: "foo".to_string(),
                data_or_elements: DataOrElements::Elements(vec![]),
                attributes: vec![],
            }
        ]);
        assert_eq!(Ok(("</elem>", expected)), res);
    }

    #[test]
    fn test_data_or_elements_with_nothing() {
        let doc = r#"</elem>"#;
        let res = data_or_elements().parse(doc);
        let expected = DataOrElements::Data("".to_string());
        assert_eq!(Ok(("</elem>", expected)), res);
    }

    #[test]
    fn test_element_with_data() {
        let doc = r#"<elem>foo</elem>"#;
        let res = element().parse(doc);
        let expected = Element {
            name: "elem".to_string(),
            data_or_elements: DataOrElements::Data("foo".to_string()),
            attributes: vec![],
        };
        assert_eq!(res, Ok(("",expected)));
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