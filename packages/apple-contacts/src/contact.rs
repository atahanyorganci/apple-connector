use crate::error::{ContactsError, ContactsResult};
#[cfg(target_os = "macos")]
use crate::{container::ContainerResolveHint, store::ContactsStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabeledStringInput {
    pub label: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostalAddressInput {
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateContactInput {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub organization_name: Option<String>,
    pub job_title: Option<String>,
    pub department_name: Option<String>,
    pub note: Option<String>,
    pub phone_numbers: Vec<LabeledStringInput>,
    pub email_addresses: Vec<LabeledStringInput>,
    pub postal_addresses: Vec<PostalAddressInput>,
    pub url_addresses: Vec<LabeledStringInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateContactInput {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub organization_name: Option<String>,
    pub job_title: Option<String>,
    pub department_name: Option<String>,
    pub note: Option<Option<String>>,
    pub phone_numbers: Option<Vec<LabeledStringInput>>,
    pub email_addresses: Option<Vec<LabeledStringInput>>,
    pub postal_addresses: Option<Vec<PostalAddressInput>>,
    pub url_addresses: Option<Vec<LabeledStringInput>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedContact {
    pub identifier: String,
}

pub(crate) fn validate_create_contact_input(input: &CreateContactInput) -> ContactsResult<()> {
    let has_name = [input.given_name.as_deref(), input.family_name.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty());
    let has_org = input
        .organization_name
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());

    if !(has_name || has_org) {
        return Err(ContactsError::ValidationFailed(
            "contact requires a name or organization".into(),
        ));
    }

    for phone in &input.phone_numbers {
        validate_labeled_value(phone, "phone number")?;
    }
    for email in &input.email_addresses {
        validate_labeled_value(email, "email address")?;
    }
    for url in &input.url_addresses {
        validate_labeled_value(url, "url")?;
    }

    Ok(())
}

pub(crate) fn validate_labeled_value(
    input: &LabeledStringInput,
    field: &str,
) -> ContactsResult<()> {
    if input.value.trim().is_empty() {
        return Err(ContactsError::ValidationFailed(format!(
            "{field} cannot be empty"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::ProtocolObject;
#[cfg(target_os = "macos")]
use objc2_contacts::{
    CNContact, CNContactStore, CNContactVCardSerialization, CNKeyDescriptor, CNLabeledValue,
    CNMutableContact, CNMutablePostalAddress, CNPhoneNumber, CNPostalAddress, CNSaveRequest,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSMutableCopying, NSString};

#[cfg(target_os = "macos")]
impl ContactsStore {
    pub async fn create_contact(
        &self,
        container_hint: ContainerResolveHint,
        input: CreateContactInput,
    ) -> ContactsResult<SavedContact> {
        self.ensure_contacts()?;
        validate_create_contact_input(&input)?;
        self.run_on_main(move |store| {
            let (container, _) = crate::container::resolve_container(store, &container_hint)?;
            let container_id = unsafe { container.identifier() };
            let contact = unsafe { CNMutableContact::new() };
            apply_create_fields(&contact, &input)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe {
                request.addContact_toContainerWithIdentifier(&contact, Some(&container_id));
            }
            execute_save(store, &request)?;
            Ok(SavedContact {
                identifier: unsafe { contact.identifier().to_string() },
            })
        })
        .await
    }

    pub async fn update_contact(
        &self,
        contact_id: &str,
        input: UpdateContactInput,
    ) -> ContactsResult<SavedContact> {
        self.ensure_contacts()?;
        let contact_id = contact_id.to_owned();
        self.run_on_main(move |store| {
            let contact = lookup_contact(store, &contact_id)?;
            let mutable = mutable_contact(&contact)?;
            apply_update_fields(&mutable, input)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.updateContact(&mutable) };
            execute_save(store, &request)?;
            Ok(SavedContact {
                identifier: unsafe { mutable.identifier().to_string() },
            })
        })
        .await
    }

    pub async fn delete_contact(&self, contact_id: &str) -> ContactsResult<()> {
        self.ensure_contacts()?;
        let contact_id = contact_id.to_owned();
        self.run_on_main(move |store| {
            let contact = lookup_contact(store, &contact_id)?;
            let mutable = mutable_contact(&contact)?;
            let request = unsafe { CNSaveRequest::new() };
            unsafe { request.deleteContact(&mutable) };
            execute_save(store, &request)
        })
        .await
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn execute_save(store: &CNContactStore, request: &CNSaveRequest) -> ContactsResult<()> {
    unsafe { store.executeSaveRequest_error(request) }.map_err(crate::error::map_cn_error)
}

#[cfg(target_os = "macos")]
pub(crate) fn lookup_contact(
    store: &CNContactStore,
    identifier: &str,
) -> ContactsResult<Retained<CNContact>> {
    let keys = contact_write_keys();
    let ns_id = NSString::from_str(identifier);
    unsafe { store.unifiedContactWithIdentifier_keysToFetch_error(&ns_id, &keys) }
        .map_err(crate::error::map_cn_error)
}

#[cfg(target_os = "macos")]
fn mutable_contact(contact: &CNContact) -> ContactsResult<Retained<CNMutableContact>> {
    Ok(contact.mutableCopy())
}

#[cfg(target_os = "macos")]
fn contact_write_keys() -> Retained<NSArray<ProtocolObject<dyn CNKeyDescriptor>>> {
    let descriptor = unsafe { CNContactVCardSerialization::descriptorForRequiredKeys() };
    NSArray::from_retained_slice(&[descriptor])
}

#[cfg(target_os = "macos")]
fn apply_create_fields(
    contact: &CNMutableContact,
    input: &CreateContactInput,
) -> ContactsResult<()> {
    set_optional_string(
        contact,
        input.given_name.as_deref(),
        |contact, value| unsafe { contact.setGivenName(value) },
    );
    set_optional_string(
        contact,
        input.family_name.as_deref(),
        |contact, value| unsafe { contact.setFamilyName(value) },
    );
    set_optional_string(
        contact,
        input.middle_name.as_deref(),
        |contact, value| unsafe { contact.setMiddleName(value) },
    );
    set_optional_string(
        contact,
        input.nickname.as_deref(),
        |contact, value| unsafe { contact.setNickname(value) },
    );
    set_optional_string(
        contact,
        input.organization_name.as_deref(),
        |contact, value| unsafe { contact.setOrganizationName(value) },
    );
    set_optional_string(
        contact,
        input.job_title.as_deref(),
        |contact, value| unsafe { contact.setJobTitle(value) },
    );
    set_optional_string(
        contact,
        input.department_name.as_deref(),
        |contact, value| unsafe { contact.setDepartmentName(value) },
    );
    set_optional_string(contact, input.note.as_deref(), |contact, value| unsafe {
        contact.setNote(value)
    });

    if !input.phone_numbers.is_empty() {
        let phones = build_phone_numbers(&input.phone_numbers)?;
        unsafe { contact.setPhoneNumbers(&phones) };
    }
    if !input.email_addresses.is_empty() {
        let emails = build_string_labeled_values(&input.email_addresses)?;
        unsafe { contact.setEmailAddresses(&emails) };
    }
    if !input.postal_addresses.is_empty() {
        let addresses = build_postal_addresses(&input.postal_addresses)?;
        unsafe { contact.setPostalAddresses(&addresses) };
    }
    if !input.url_addresses.is_empty() {
        let urls = build_string_labeled_values(&input.url_addresses)?;
        unsafe { contact.setUrlAddresses(&urls) };
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_update_fields(
    contact: &CNMutableContact,
    input: UpdateContactInput,
) -> ContactsResult<()> {
    if let Some(given_name) = input.given_name {
        let ns = NSString::from_str(&given_name);
        unsafe { contact.setGivenName(&ns) };
    }
    if let Some(family_name) = input.family_name {
        let ns = NSString::from_str(&family_name);
        unsafe { contact.setFamilyName(&ns) };
    }
    if let Some(middle_name) = input.middle_name {
        let ns = NSString::from_str(&middle_name);
        unsafe { contact.setMiddleName(&ns) };
    }
    if let Some(nickname) = input.nickname {
        let ns = NSString::from_str(&nickname);
        unsafe { contact.setNickname(&ns) };
    }
    if let Some(organization_name) = input.organization_name {
        let ns = NSString::from_str(&organization_name);
        unsafe { contact.setOrganizationName(&ns) };
    }
    if let Some(job_title) = input.job_title {
        let ns = NSString::from_str(&job_title);
        unsafe { contact.setJobTitle(&ns) };
    }
    if let Some(department_name) = input.department_name {
        let ns = NSString::from_str(&department_name);
        unsafe { contact.setDepartmentName(&ns) };
    }
    if let Some(note) = input.note {
        match note {
            Some(value) => {
                let ns = NSString::from_str(&value);
                unsafe { contact.setNote(&ns) };
            }
            None => unsafe { contact.setNote(&NSString::from_str("")) },
        }
    }
    if let Some(phone_numbers) = input.phone_numbers {
        let phones = build_phone_numbers(&phone_numbers)?;
        unsafe { contact.setPhoneNumbers(&phones) };
    }
    if let Some(email_addresses) = input.email_addresses {
        let emails = build_string_labeled_values(&email_addresses)?;
        unsafe { contact.setEmailAddresses(&emails) };
    }
    if let Some(postal_addresses) = input.postal_addresses {
        let addresses = build_postal_addresses(&postal_addresses)?;
        unsafe { contact.setPostalAddresses(&addresses) };
    }
    if let Some(url_addresses) = input.url_addresses {
        let urls = build_string_labeled_values(&url_addresses)?;
        unsafe { contact.setUrlAddresses(&urls) };
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_optional_string<F>(contact: &CNMutableContact, value: Option<&str>, setter: F)
where
    F: FnOnce(&CNMutableContact, &NSString),
{
    if let Some(value) = value {
        let ns = NSString::from_str(value);
        setter(contact, &ns);
    }
}

#[cfg(target_os = "macos")]
fn build_phone_numbers(
    inputs: &[LabeledStringInput],
) -> ContactsResult<Retained<NSArray<CNLabeledValue<CNPhoneNumber>>>> {
    let mut values = Vec::with_capacity(inputs.len());
    for input in inputs {
        validate_labeled_value(input, "phone number")?;
        let string = NSString::from_str(&input.value);
        let phone = unsafe { CNPhoneNumber::phoneNumberWithStringValue(&string) }
            .ok_or_else(|| ContactsError::ValidationFailed("invalid phone number".into()))?;
        let label = input.label.as_ref().map(|value| NSString::from_str(value));
        let labeled =
            unsafe { CNLabeledValue::labeledValueWithLabel_value(label.as_deref(), &*phone) };
        values.push(labeled);
    }
    Ok(NSArray::from_retained_slice(&values))
}

#[cfg(target_os = "macos")]
fn build_string_labeled_values(
    inputs: &[LabeledStringInput],
) -> ContactsResult<Retained<NSArray<CNLabeledValue<NSString>>>> {
    let mut values = Vec::with_capacity(inputs.len());
    for input in inputs {
        validate_labeled_value(input, "value")?;
        let value = NSString::from_str(&input.value);
        let label = input.label.as_ref().map(|value| NSString::from_str(value));
        let labeled =
            unsafe { CNLabeledValue::labeledValueWithLabel_value(label.as_deref(), &*value) };
        values.push(labeled);
    }
    Ok(NSArray::from_retained_slice(&values))
}

#[cfg(target_os = "macos")]
fn build_postal_addresses(
    inputs: &[PostalAddressInput],
) -> ContactsResult<Retained<NSArray<CNLabeledValue<CNPostalAddress>>>> {
    let mut values = Vec::with_capacity(inputs.len());
    for input in inputs {
        let address = unsafe { CNMutablePostalAddress::new() };
        if let Some(street) = &input.street {
            let ns = NSString::from_str(street);
            unsafe { address.setStreet(&ns) };
        }
        if let Some(city) = &input.city {
            let ns = NSString::from_str(city);
            unsafe { address.setCity(&ns) };
        }
        if let Some(state) = &input.state {
            let ns = NSString::from_str(state);
            unsafe { address.setState(&ns) };
        }
        if let Some(postal_code) = &input.postal_code {
            let ns = NSString::from_str(postal_code);
            unsafe { address.setPostalCode(&ns) };
        }
        if let Some(country) = &input.country {
            let ns = NSString::from_str(country);
            unsafe { address.setCountry(&ns) };
        }
        let label = input.label.as_ref().map(|value| NSString::from_str(value));
        let labeled = unsafe {
            CNLabeledValue::<CNPostalAddress>::labeledValueWithLabel_value(
                label.as_deref(),
                &*address,
            )
        };
        values.push(labeled);
    }
    Ok(NSArray::from_retained_slice(&values))
}

#[cfg(test)]
mod tests {
    use super::{
        CreateContactInput, LabeledStringInput, UpdateContactInput, validate_create_contact_input,
        validate_labeled_value,
    };
    use crate::ContactsError;

    #[test]
    fn create_requires_name_or_organization() {
        let input = CreateContactInput {
            given_name: None,
            family_name: None,
            middle_name: None,
            nickname: None,
            organization_name: None,
            job_title: None,
            department_name: None,
            note: None,
            phone_numbers: Vec::new(),
            email_addresses: Vec::new(),
            postal_addresses: Vec::new(),
            url_addresses: Vec::new(),
        };
        assert_eq!(
            validate_create_contact_input(&input).unwrap_err(),
            ContactsError::ValidationFailed("contact requires a name or organization".into())
        );
    }

    #[test]
    fn organization_only_contact_is_valid() {
        let input = CreateContactInput {
            given_name: None,
            family_name: None,
            middle_name: None,
            nickname: None,
            organization_name: Some("Acme Corp".into()),
            job_title: None,
            department_name: None,
            note: None,
            phone_numbers: Vec::new(),
            email_addresses: Vec::new(),
            postal_addresses: Vec::new(),
            url_addresses: Vec::new(),
        };
        assert!(validate_create_contact_input(&input).is_ok());
    }

    #[test]
    fn empty_labeled_value_is_invalid() {
        let input = LabeledStringInput {
            label: Some("mobile".into()),
            value: "   ".into(),
        };
        assert_eq!(
            validate_labeled_value(&input, "phone number").unwrap_err(),
            ContactsError::ValidationFailed("phone number cannot be empty".into())
        );
    }

    #[test]
    fn update_input_can_clear_note() {
        let input = UpdateContactInput {
            given_name: None,
            family_name: None,
            middle_name: None,
            nickname: None,
            organization_name: None,
            job_title: None,
            department_name: None,
            note: Some(None),
            phone_numbers: None,
            email_addresses: None,
            postal_addresses: None,
            url_addresses: None,
        };
        assert!(input.note.is_some());
    }
}
