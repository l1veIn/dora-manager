import { addMessages, init, getLocaleFromNavigator, locale } from 'svelte-i18n';
import en from './locales/en.json';
import zhCN from './locales/zh-CN.json';

addMessages('en', en);
addMessages('zh-CN', zhCN);

// Initialize i18n
init({
    fallbackLocale: 'en',
    initialLocale: typeof window !== "undefined" ? window.localStorage.getItem("dm-language") || getLocaleFromNavigator() : 'en',
});

// Subscribe to locale changes to persist to localStorage
if (typeof window !== "undefined") {
    locale.subscribe((newLocale) => {
        if (newLocale) {
            window.localStorage.setItem("dm-language", newLocale);
        }
    });
}
