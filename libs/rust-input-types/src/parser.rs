// =============================================================================
// 🔄 Form Parser — parsează URL-encoded body în vector de cîmpuri
// =============================================================================
// Folosit de ValidatedForm extractor pentru a transforma body-ul HTTP
// într-o structură de date validată.
//
// Standard: OWASP ASVS V5.1 (Input Validation)
// =============================================================================

use crate::error::InputError;

/// Un cîmp dintr-un formular URL-encoded
#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub value: String,
}

/// Parsează un URL-encoded body în vector de FormField.
/// Suportă: `name1=value1&name2=value2`
pub fn parse_form(body: &str) -> Vec<FormField> {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let name = parts.next()?;
            let name = url_decode(name);
            let value = parts.next().unwrap_or("");
            let value = url_decode(value);
            Some(FormField { name, value })
        })
        .collect()
}

/// URL-decode simplu (decodează %XX și +)
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '+' {
            result.push(' ');
        } else if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Extrage valoarea unui cîmp după nume.
pub fn get_field<'a>(fields: &'a [FormField], name: &str) -> Result<&'a str, InputError> {
    fields
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.value.as_str())
        .ok_or_else(|| InputError::MissingField(name.to_string()))
}

/// Parsează body-ul și aplică o funcție de validare.
pub fn parse_form_into<T>(body: &str, f: impl FnOnce(&[FormField]) -> Result<T, InputError>) -> Result<T, InputError> {
    let fields = parse_form(body);
    f(&fields)
}

/// La fel ca parse_form_into, dar suportă și JSON content-type
pub fn parse_any_into<T>(body: &str, f: impl FnOnce(&[FormField]) -> Result<T, InputError>) -> Result<T, InputError> {
    // Încearcă JSON first
    if body.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
            let fields = json_to_fields(&val);
            return f(&fields);
        }
    }
    parse_form_into(body, f)
}

/// Convertește un JSON Value în vector de FormField (doar primul nivel)
fn json_to_fields(val: &serde_json::Value) -> Vec<FormField> {
    match val {
        serde_json::Value::Object(map) => {
            map.iter().map(|(k, v)| {
                FormField {
                    name: k.clone(),
                    value: v.as_str().unwrap_or("").to_string(),
                }
            }).collect()
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_form() {
        let fields = parse_form("name=test&qty=3");
        assert_eq!(fields.len(), 2);
        assert_eq!(get_field(&fields, "name"), Ok("test"));
        assert_eq!(get_field(&fields, "qty"), Ok("3"));
    }

    #[test]
    fn test_url_decoded() {
        let fields = parse_form("msg=hello+world&name=Ion+Popa");
        assert_eq!(get_field(&fields, "msg"), Ok("hello world"));
        assert_eq!(get_field(&fields, "name"), Ok("Ion Popa"));
    }

    #[test]
    fn test_empty_form() {
        let fields = parse_form("");
        assert!(fields.is_empty());
    }

    #[test]
    fn test_missing_field() {
        let fields = parse_form("a=1");
        assert_eq!(get_field(&fields, "b"), Err(InputError::MissingField("b".to_string())));
    }

    #[test]
    fn test_parse_form_into() {
        let result = parse_form_into("x=10&y=20", |fields| {
            let x = get_field(fields, "x")?;
            Ok(x.to_string())
        });
        assert_eq!(result.unwrap(), "10");
    }
}
