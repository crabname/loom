#[derive(Debug, Clone)]
pub struct FormField {
    pub enabled: bool,
    pub name: String,
    pub value: String,
}

pub type KeyValueField = FormField;

impl FormField {
    pub fn empty() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            value: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MultipartFieldType {
    #[default]
    Text,
    File,
}

impl MultipartFieldType {
    pub const ALL: [Self; 2] = [Self::Text, Self::File];

    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::File => "file",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|field_type| field_type.label() == label)
    }
}

#[derive(Debug, Clone)]
pub struct MultipartField {
    pub enabled: bool,
    pub name: String,
    pub value: String,
    pub field_type: MultipartFieldType,
    pub content_type: String,
}

impl MultipartField {
    pub fn empty() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            value: String::new(),
            field_type: MultipartFieldType::Text,
            content_type: String::new(),
        }
    }
}

pub fn default_key_value_fields() -> Vec<KeyValueField> {
    vec![KeyValueField::empty()]
}

pub fn default_form_fields() -> Vec<FormField> {
    vec![FormField::empty()]
}

pub fn default_multipart_fields() -> Vec<MultipartField> {
    vec![MultipartField::empty()]
}
