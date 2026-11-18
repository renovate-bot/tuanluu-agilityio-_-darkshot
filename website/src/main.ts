import "./styles/main.css";
import { initCompare, initHero, initR2, initRedact, initSnap, initTabs, injectScenes } from "./demos";
import { initI18n } from "./i18n";
import { initRelease } from "./release";
import { initFeatureNav, initNav, initReveal, initYear } from "./ui";

injectScenes();
initI18n();
initNav();
initHero();
initSnap();
initCompare();
initRedact();
initTabs();
initR2();
initFeatureNav();
initReveal();
initYear();
void initRelease();
