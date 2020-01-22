use super::fetch::build_url;
use html5ever::interface::Attribute;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use rcdom::{Handle, NodeData, RcDom};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub href: Option<String>,
    pub resolved: Option<String>,
}

pub fn parse_html(source: &str) -> RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut source.as_bytes())
        .unwrap()
}

pub fn get_links(handle: Handle, domain: &str, page: &str) -> Vec<Arc<Link>> {
    let mut links = vec![];
    let mut anchor_tags = vec![];

    get_elements_by_name(handle, "a", &mut anchor_tags);

    for node in anchor_tags {
        if let NodeData::Element { ref attrs, .. } = node {
            let mut link_attrs = HashMap::new();
            link_attrs.insert("page".to_string(), page.to_string());
            for attr in attrs.borrow().iter() {
                let Attribute {
                    ref name,
                    ref value,
                } = *attr;
                link_attrs.insert(name.local.to_string(), value.to_string());
            }

            if let Some(href) = link_attrs.get("href") {
                if let Ok(url) = build_url(domain, href) {
                    match url.scheme() {
                        "http" | "https" => {
                            link_attrs.insert("resolved".to_string(), url.to_string());
                            let link = Link {
                                href: match link_attrs.get("href") {
                                    Some(val) => Some(val.to_owned()),
                                    _ => None,
                                },
                                resolved: match link_attrs.get("resolved") {
                                    Some(val) => Some(val.to_owned()),
                                    _ => None,
                                },
                            };
                            links.push(Arc::new(link));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    links
}

fn get_elements_by_name(handle: Handle, element_name: &str, out: &mut Vec<NodeData>) {
    let node = handle;

    if let NodeData::Element {
        ref name,
        ref attrs,
        ref template_contents,
        ..
    } = node.data
    {
        if &*(name.local) == element_name {
            out.push(NodeData::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                template_contents: template_contents.clone(),
                mathml_annotation_xml_integration_point: false,
            });
        }
    }

    for n in node.children.borrow().iter() {
        get_elements_by_name(n.clone(), element_name, out);
    }
}
