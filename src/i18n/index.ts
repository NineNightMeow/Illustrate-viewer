import { createI18n } from "vue-i18n";
import enUS from "./en-US";
import jaJP from "./ja-JP";
import zhCN from "./zh-CN";

export const messages = {
  "en-US": enUS,
  "ja-JP": jaJP,
  "zh-CN": zhCN,
};

export type AppLocale = keyof typeof messages;

export const i18n = createI18n({
  legacy: false,
  locale: "en-US",
  fallbackLocale: "en-US",
  messages,
});
