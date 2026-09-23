//! # OpenPet Internationalization
//!
//! Dual-language support for Turkish (`tr-TR`) and English (`en-US`) with fallback logic.

use openpet_types::SupportedLocale;
use std::collections::HashMap;

pub struct I18nManager {
    current_locale: SupportedLocale,
    en_strings: HashMap<&'static str, &'static str>,
    tr_strings: HashMap<&'static str, &'static str>,
}

impl Default for I18nManager {
    fn default() -> Self {
        Self::new(SupportedLocale::EnUs)
    }
}

impl I18nManager {
    pub fn new(locale: SupportedLocale) -> Self {
        let mut en = HashMap::new();
        let mut tr = HashMap::new();

        // Application Common
        en.insert("app.title", "OpenPet");
        tr.insert("app.title", "OpenPet");

        en.insert("app.subtitle", "Your Open-Source Desktop Pet & Companion");
        tr.insert("app.subtitle", "Açık Kaynak Masaüstü Evcil Hayvanınız");

        // Tray Strings
        en.insert("tray.open_control", "Open Control Center");
        tr.insert("tray.open_control", "Kontrol Merkezini Aç");

        en.insert("tray.show_hide_pet", "Show/Hide Pet");
        tr.insert("tray.show_hide_pet", "Peti Göster/Gizle");

        en.insert("tray.chat", "Chat");
        tr.insert("tray.chat", "Sohbet");

        en.insert("tray.privacy_mode", "Privacy Mode (Pause Capture)");
        tr.insert("tray.privacy_mode", "Gizlilik Modu (Yakalamayı Duraklat)");

        en.insert("tray.pause_pet", "Pause Pet");
        tr.insert("tray.pause_pet", "Peti Duraklat");

        en.insert("tray.settings", "Settings");
        tr.insert("tray.settings", "Ayarlar");

        en.insert("tray.exit", "Exit OpenPet");
        tr.insert("tray.exit", "OpenPet'ten Çık");

        // Navigation Strings
        en.insert("nav.home", "Home");
        tr.insert("nav.home", "Ana Sayfa");

        en.insert("nav.pet", "Pet");
        tr.insert("nav.pet", "Evcil Hayvan");

        en.insert("nav.create_pet", "Create Pet");
        tr.insert("nav.create_pet", "Pet Oluştur");

        en.insert("nav.chat", "Chat");
        tr.insert("nav.chat", "Sohbet");

        en.insert("nav.reminders", "Reminders");
        tr.insert("nav.reminders", "Hatırlatıcılar");

        en.insert("nav.memory", "Memory");
        tr.insert("nav.memory", "Hafıza");

        en.insert("nav.plugins", "Plugins");
        tr.insert("nav.plugins", "Eklentiler");

        en.insert("nav.privacy", "Privacy");
        tr.insert("nav.privacy", "Gizlilik");

        en.insert("nav.ai_providers", "AI Providers");
        tr.insert("nav.ai_providers", "Yapay Zeka Sağlayıcıları");

        en.insert("nav.settings", "Settings");
        tr.insert("nav.settings", "Ayarlar");

        en.insert("nav.about", "About");
        tr.insert("nav.about", "Hakkında");

        // Behaviors
        en.insert("behavior.idle", "Relaxing");
        tr.insert("behavior.idle", "Dinleniyor");

        en.insert("behavior.walk", "Walking");
        tr.insert("behavior.walk", "Yürüyor");

        en.insert("behavior.sleep", "Sleeping");
        tr.insert("behavior.sleep", "Uyuyor");

        en.insert("behavior.play", "Playing");
        tr.insert("behavior.play", "Oynuyor");

        Self {
            current_locale: locale,
            en_strings: en,
            tr_strings: tr,
        }
    }

    pub fn set_locale(&mut self, locale: SupportedLocale) {
        self.current_locale = locale;
    }

    pub fn current_locale(&self) -> SupportedLocale {
        self.current_locale
    }

    /// Retrieve localized text for key, falling back to English if missing or key itself.
    pub fn translate<'a>(&self, key: &'a str) -> &'a str {
        match self.current_locale {
            SupportedLocale::TrTr => self
                .tr_strings
                .get(key)
                .copied()
                .or_else(|| self.en_strings.get(key).copied())
                .unwrap_or(key),
            SupportedLocale::EnUs => self.en_strings.get(key).copied().unwrap_or(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_and_fallback() {
        let mut i18n = I18nManager::new(SupportedLocale::EnUs);
        assert_eq!(i18n.translate("tray.exit"), "Exit OpenPet");

        i18n.set_locale(SupportedLocale::TrTr);
        assert_eq!(i18n.translate("tray.exit"), "OpenPet'ten Çık");

        // Fallback test
        assert_eq!(i18n.translate("non_existent_key"), "non_existent_key");
    }
}
