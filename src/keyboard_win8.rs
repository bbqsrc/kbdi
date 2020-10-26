use crate::keyboard::{Error, KeyboardRegKey};
use crate::language::LanguageRegKey;
use crate::platform::*;
use crate::types::*;
use registry::{Data, Hive, RegKey, Security};
use std::{
    collections::hash_map::HashMap,
    convert::{TryFrom, TryInto},
};

fn enabled_input_methods() -> InputList {
    let langs = crate::enabled_languages().unwrap();
    let mut imes: Vec<String> = vec![];
    for lang in langs {
        imes.append(&mut bcp47langs::get_user_language_input_methods(&lang).unwrap());
    }
    InputList::try_from(imes).unwrap()
}

pub fn enable(tag: &str, product_code: &str, lang_name: Option<&str>) -> Result<(), Error> {
    log::info!("Enabling '{}' with product code '{}'", tag, product_code);
    log::info!("Lang name: {:?}", lang_name);

    let record = match KeyboardRegKey::find_by_product_code(product_code) {
        Some(v) => v,
        None => return Err(Error::NotFound),
    };

    // Check language is enabled or LCID check will fail
    log::info!("Enabling language by tag");
    crate::enable_language(tag).unwrap();

    // Get all languages and keyboards
    let mut keyboards = crate::win8::enabled_keyboards()
        .unwrap()
        .into_iter()
        .collect::<HashMap<_, _>>();

    let lcid = bcp47langs::lcid_from_bcp47(tag).unwrap();
    let tip = format!("{:04X}:{}", lcid, record.regkey_id());
    keyboards
        .entry(tag.to_string())
        .and_modify(|x| x.push(tip.clone()))
        .or_insert(vec![tip.clone()]);

    // Remove all inputs internal
    bcp47langs::remove_inputs_for_all_languages().unwrap();

    // Build input method list
    let mut first = true;
    for (lang_tag, tips) in keyboards {
        let _lcid = match bcp47langs::lcid_from_bcp47(&lang_tag) {
            Some(v) => v,
            None => continue,
        };

        let inputs = InputList::try_from(tips).unwrap();

        // Flag 256 seems to clear everything.
        let flag = if first { 256 } else { 0 };
        first = false;

        input::install_layout(inputs, flag).unwrap();
    }

    log::info!("Regenerating registry for keyboards");
    regenerate_registry();

    Ok(())
}

pub fn remove_invalid_kbids() {
    let installed_imes: Vec<String> = KeyboardRegKey::installed()
        .iter()
        .map(|x| x.regkey_id().to_owned())
        .collect();

    let enabled_imes = enabled_input_methods();
    let filtered_imes: Vec<InputListItem> = enabled_imes
        .into_inner()
        .into_iter()
        .filter(|i| {
            let kbid = i.kbid().to_string().to_lowercase();
            // Only handle custom keyboards
            if kbid.starts_with("a") {
                return true;
            }
            installed_imes.contains(&kbid)
        })
        .collect();

    bcp47langs::remove_inputs_for_all_languages().unwrap();
    input::install_layout(InputList::from(filtered_imes), 0).unwrap();
}

pub fn regenerate_registry() {
    let user_profile_key = Hive::CurrentUser
        .open(r"Control Panel\International\User Profile", Security::Read)
        .unwrap();
    let substitutes_key = Hive::CurrentUser
        .open(
            r"Keyboard Layout\Substitutes",
            Security::Read | Security::Write,
        )
        .unwrap();
    let preload_key = Hive::CurrentUser
        .open(r"Keyboard Layout\Preload", Security::Read | Security::Write)
        .unwrap();

    regenerate_given_registry(user_profile_key, substitutes_key, preload_key);
}

fn enable_default_user_lang(tag: &str) {
    let default_user_registry = registry::Hive::load_file(
        r"C:\Users\Default\NTUSER.DAT",
        Security::Read | Security::Write,
    )
    .unwrap();
    let language_key = LanguageRegKey::find_by_tag(tag).unwrap().regkey;
    let substitutes_key = Hive::CurrentUser
        .open(
            r"Keyboard Layout\Substitutes",
            Security::Read | Security::Write,
        )
        .unwrap();

    let default_user_profile_key = default_user_registry
        .create(
            format!(r"Control Panel\International\User Profile\{}", tag),
            Security::Read | Security::Write,
        )
        .unwrap();
    let default_substitutes_key = default_user_registry
        .open(
            r"Keyboard Layout\Substitutes",
            Security::Read | Security::Write,
        )
        .unwrap();

    for value in language_key.values() {
        let value = value.unwrap();
        default_user_profile_key
            .set_value(value.name(), value.data())
            .unwrap();
    }

    for value in substitutes_key.values() {
        let value = value.unwrap();
        default_substitutes_key
            .set_value(value.name(), value.data())
            .unwrap();
    }

    let preload_key = default_user_registry
        .open(r"Keyboard Layout\Preload", Security::Read | Security::Write)
        .unwrap();
    regenerate_given_registry(default_user_profile_key, substitutes_key, preload_key);
}

fn regenerate_given_registry(
    user_profile_key: RegKey,
    substitutes_key: RegKey,
    preload_key: RegKey,
) {
    log::debug!("regenerate_given_registry");
    let lang_keys: Vec<_> = user_profile_key
        .keys()
        .map(|k| k.unwrap().open(Security::Read).unwrap())
        .collect();

    log::trace!("Lang keys: {:?}", lang_keys);

    // Get known keyboard ids from Control Panel configured language list
    let keyboard_ids: Vec<_> = lang_keys
        .iter()
        .flat_map(|k| k.values())
        .map(|v| v.unwrap().name().to_string_lossy())
        .filter(|n| n.contains(":"))
        .map(|v| InputListItem::try_from(&*v))
        // .map(|n| n.split(":").last().unwrap().to_string())
        .filter_map(Result::ok)
        .collect();

    log::trace!("Keyboard IDs: {:?}", &keyboard_ids);

    // Get all substitutes into a list
    let subs = substitutes_key
        .values()
        .filter_map(Result::ok)
        .map(|x| {
            let x = x.into_inner();
            (x.0.to_string_lossy(), x.1.to_string())
        })
        .collect::<Vec<_>>();

    log::trace!("Substitutions: {:?}", &subs);

    // Clean up all invalid substitutions
    for (value_id, kbd_id) in subs.iter() {
        if keyboard_ids
            .iter()
            .find(|x| &format!("{:08x}", x.tip_id) == kbd_id)
            .is_none()
        {
            log::debug!("Deleting substitute: {:?}", value_id);
            substitutes_key.delete_value(value_id).unwrap();
        }
    }

    // Delete all preload values
    for value in preload_key.values() {
        let name = value.unwrap().name().to_owned();
        preload_key.delete_value(name).unwrap();
    }

    log::trace!("Cleared all preload keys");

    // Check if substitutes contains lang_id
    for (i, item) in keyboard_ids.iter().enumerate() {
        let lcid = format!("{:08x}", item.lang_id);
        let tip = format!("{:08x}", item.tip_id);

        let value = if subs.iter().find(|sub| sub.0 == lcid).is_some() {
            log::trace!("Adding substitute lcid: {}", &lcid);
            lcid.try_into().unwrap()
        } else {
            log::trace!("Adding TIP: {}", &tip);
            tip.try_into().unwrap()
        };

        preload_key
            .set_value((i + 1).to_string(), &Data::String(value))
            .unwrap();
    }
}