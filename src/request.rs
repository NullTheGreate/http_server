use std::collections::HashMap;

pub struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
}

impl Request {
    pub fn parse(raw: &str) -> Option<Self> {
        let lines: Vec<&str> = raw.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();
        let body = lines
            .iter()
            .skip_while(|line| !line.is_empty())
            .skip(1)
            .map(|line| *line) // Dereference &&str to &str
            .collect::<Vec<&str>>()
            .join("\n");

        Some(Request { method, path, body })
    }

    pub fn parse_body(&self) -> HashMap<String, String> {
        self.body
            .split('&')
            .filter_map(|pair| {
                let kv: Vec<&str> = pair.splitn(2, '=').collect();
                if kv.len() == 2 {
                    Some((kv[0].to_string(), kv[1].to_string()))
                } else {
                    None
                }
            })
            .collect()
    }
}
