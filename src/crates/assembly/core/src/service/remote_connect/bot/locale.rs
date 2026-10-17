pub use openbitfun_services_integrations::remote_connect::bot::{
    fmt_count, strings_for, BotLanguage, BotStrings,
};

pub async fn current_bot_language() -> BotLanguage {
    match crate::service::config::get_app_language().await {
        crate::service::LocaleId::ZhCN => BotLanguage::ZhCN,
        crate::service::LocaleId::ZhTW => BotLanguage::ZhTW,
        crate::service::LocaleId::EnUS => BotLanguage::EnUS,
        // fr-FR bot copy has not landed yet; serve the contract fallback
        // locale until the fr-FR surface effort adds BotLanguage::FrFR.
        crate::service::LocaleId::FrFR => BotLanguage::EnUS,
    }
}
