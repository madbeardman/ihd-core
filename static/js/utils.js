import { state } from "./state.js";

export function isDevMode() {
    const dashboard = document.getElementById("dashboard");
    return dashboard?.dataset.devMode === "true";
}

export function formatPrice(value) {
    return `${value.toFixed(2)}p`;
}

export function formatHourLabel(isoString) {
    const date = new Date(isoString);
    return date.getHours().toString().padStart(2, "0");
}

export function formatClock(now = new Date()) {
    return now.toLocaleTimeString("en-GB", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    });
}

export function formatLastUpdated(now = new Date()) {
    return now.toLocaleTimeString("en-GB", {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    });
}

export function formatGbp(value) {
    if (value == null) return "£--";
    return `£${value.toFixed(2)}`;
}

export function formatHistoryCost(value) {
    if (value == null) return "£--";
    return `£${value.toFixed(3)}`;
}

export function formatHistoryKwh(value) {
    if (value == null) return "-- kWh";
    return `${value.toFixed(3)} kWh`;
}

export function getHistoryDisplayValue(rawValue) {
    if (state.historyMetric === "cost") {
        return formatHistoryCost(rawValue);
    }

    return formatHistoryKwh(rawValue);
}

export function roundAxisMax(value) {
    return Math.ceil(value * 100) / 100;
}

export function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
}