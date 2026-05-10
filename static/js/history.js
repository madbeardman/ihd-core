import { state } from "./state.js";
import {
    formatGbp,
    formatHistoryCost,
    formatHistoryKwh,
    getHistoryDisplayValue,
} from "./utils.js";

function getHistorySlotValue(slot) {
    if (!slot) return 0;

    if (state.historyMetric === "cost") {
        return typeof slot.cost_gbp === "number" ? slot.cost_gbp : 0;
    }

    return typeof slot.consumption_kwh === "number" ? slot.consumption_kwh : 0;
}

function getHistorySummaryValue(summary) {
    if (!summary) return 0;

    if (state.historyMetric === "cost") {
        return typeof summary.total_cost_gbp === "number" ? summary.total_cost_gbp : 0;
    }

    return typeof summary.total_consumption_kwh === "number"
        ? summary.total_consumption_kwh
        : 0;
}

function formatHistoryDateLabel(isoDate) {
    const date = new Date(`${isoDate}T12:00:00`);
    return date.toLocaleDateString("en-GB", {
        weekday: "short",
        day: "numeric",
        month: "long",
        year: "numeric",
    });
}

function formatWeekRangeLabel(startIsoDate, endIsoDate) {
    const start = new Date(`${startIsoDate}T12:00:00`);
    const end = new Date(`${endIsoDate}T12:00:00`);

    const sameMonth =
        start.getMonth() === end.getMonth() &&
        start.getFullYear() === end.getFullYear();

    if (sameMonth) {
        return `${start.toLocaleDateString("en-GB", {
            day: "numeric",
        })}–${end.toLocaleDateString("en-GB", {
            day: "numeric",
            month: "long",
            year: "numeric",
        })}`;
    }

    return `${start.toLocaleDateString("en-GB", {
        day: "numeric",
        month: "short",
    })} – ${end.toLocaleDateString("en-GB", {
        day: "numeric",
        month: "short",
        year: "numeric",
    })}`;
}

function formatMonthRangeLabel(startIsoDate, endIsoDate) {
    const start = new Date(`${startIsoDate}T12:00:00`);
    const end = new Date(`${endIsoDate}T12:00:00`);

    const isWholeMonth =
        start.getDate() === 1 &&
        end.getMonth() === start.getMonth() &&
        end.getFullYear() === start.getFullYear();

    if (isWholeMonth || start.getMonth() === end.getMonth()) {
        return start.toLocaleDateString("en-GB", {
            month: "long",
            year: "numeric",
        });
    }

    return `${start.toLocaleDateString("en-GB", {
        month: "short",
        year: "numeric",
    })} – ${end.toLocaleDateString("en-GB", {
        month: "short",
        year: "numeric",
    })}`;
}

function formatDateForApi(date) {
    return date.toISOString().slice(0, 10);
}

function parseIsoDateToLocalDate(isoDate) {
    return new Date(`${isoDate}T12:00:00`);
}

function getTodayIsoDate() {
    return formatDateForApi(new Date());
}

function getYesterdayIsoDate() {
    const d = new Date();
    d.setDate(d.getDate() - 1);
    return formatDateForApi(d);
}

function shiftSelectedDate(days) {
    if (!state.historySelectedDate) {
        state.historySelectedDate = getTodayIsoDate();
    }

    const current = parseIsoDateToLocalDate(state.historySelectedDate);
    current.setDate(current.getDate() + days);
    state.historySelectedDate = formatDateForApi(current);
}

function shiftSelectedMonth(months) {
    if (!state.historySelectedDate) {
        state.historySelectedDate = getYesterdayIsoDate();
    }

    const current = parseIsoDateToLocalDate(state.historySelectedDate);
    const originalDay = current.getDate();

    current.setDate(1);
    current.setMonth(current.getMonth() + months);

    const lastDayOfTargetMonth = new Date(
        current.getFullYear(),
        current.getMonth() + 1,
        0,
    ).getDate();

    current.setDate(Math.min(originalDay, lastDayOfTargetMonth));
    state.historySelectedDate = formatDateForApi(current);
}

function syncRangeButtons() {
    document.getElementById("history-range-day")?.classList.toggle(
        "active",
        state.historyRange === "day",
    );

    document.getElementById("history-range-week")?.classList.toggle(
        "active",
        state.historyRange === "week",
    );

    document.getElementById("history-range-month")?.classList.toggle(
        "active",
        state.historyRange === "month",
    );
}

function syncMetricButtons() {
    document.getElementById("history-metric-cost")?.classList.toggle(
        "active",
        state.historyMetric === "cost",
    );
    document.getElementById("history-metric-kwh")?.classList.toggle(
        "active",
        state.historyMetric === "kwh",
    );
}

function updateNavigationButtons() {
    const nextButton = document.getElementById("history-next-button");

    if (!nextButton || !state.historySelectedDate) return;

    nextButton.disabled = state.historySelectedDate >= getYesterdayIsoDate();
}

export async function loadHistoryModalPartial() {
    const root = document.getElementById("history-modal-root");
    if (!root) return;

    const response = await fetch("/static/partials/history-modal.html", {
        headers: { Accept: "text/html" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load history modal partial: HTTP ${response.status}`);
    }

    root.innerHTML = await response.text();
}

export function openHistoryModal() {
    const modal = document.getElementById("history-modal");
    const backdrop = document.getElementById("history-backdrop");

    if (!modal || !backdrop) return;

    if (!state.historySelectedDate) {
        const yesterday = new Date();
        yesterday.setDate(yesterday.getDate() - 1);
        state.historySelectedDate = formatDateForApi(yesterday);
    }

    modal.removeAttribute("hidden");
    backdrop.removeAttribute("hidden");

    syncMetricButtons();
    syncRangeButtons();
    updateNavigationButtons();

    loadHistory();
}

export function closeHistoryModal() {
    const modal = document.getElementById("history-modal");
    const backdrop = document.getElementById("history-backdrop");

    if (!modal || !backdrop) return;

    modal.setAttribute("hidden", "");
    backdrop.setAttribute("hidden", "");
}

function renderHistoryChart(chartId, axisId, slots, fuel, yMaxId) {
    const chart = document.getElementById(chartId);
    const axis = document.getElementById(axisId);

    if (!chart || !axis) return;

    chart.innerHTML = "";
    axis.innerHTML = "";

    if (!slots || slots.length === 0) {
        chart.innerHTML = "<div style='color: var(--muted);'>No data</div>";
        return;
    }

    const values = slots.map((slot) => getHistorySlotValue(slot));
    const maxValue = Math.max(...values, 0.001);

    const yMaxEl = document.getElementById(yMaxId);
    if (yMaxEl) {
        yMaxEl.textContent = getHistoryDisplayValue(maxValue);
    }

    slots.forEach((slot, index) => {
        const rawValue = getHistorySlotValue(slot);
        const ratio = rawValue / maxValue;
        const heightPercent = Math.max(6, ratio * 100);

        const bar = document.createElement("button");
        bar.type = "button";
        bar.className = `history-bar ${fuel}`;
        bar.style.height = `${heightPercent}%`;

        const labelValue =
            state.historyMetric === "cost"
                ? formatHistoryCost(rawValue)
                : formatHistoryKwh(rawValue);

        const start = new Date(slot.interval_start);
        const end = new Date(slot.interval_end);

        const startText = start.toLocaleTimeString("en-GB", {
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
        });

        const endText = end.toLocaleTimeString("en-GB", {
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
        });

        bar.dataset.value = labelValue;
        bar.title = `${labelValue}\n${startText} → ${endText}`;
        bar.setAttribute("aria-label", `${fuel} ${labelValue}, ${startText} to ${endText}`);

        bar.addEventListener("click", () => {
            const existing = chart.querySelector(".history-bar.selected");
            if (existing && existing !== bar) {
                existing.classList.remove("selected");
            }

            bar.classList.toggle("selected");
        });

        chart.appendChild(bar);

        if (start.getMinutes() === 0) {
            const label = document.createElement("div");
            label.className = "history-time-label";
            label.textContent = start.getHours().toString().padStart(2, "0");
            label.style.left = `${(index / slots.length) * 100}%`;
            axis.appendChild(label);
        }
    });
}

function renderHistoryAggregateChart(chartId, axisId, items, fuel, yMaxId) {
    const chart = document.getElementById(chartId);
    const axis = document.getElementById(axisId);

    if (!chart || !axis) return;

    chart.innerHTML = "";
    axis.innerHTML = "";

    if (!items || items.length === 0) {
        chart.innerHTML = "<div style='color: var(--muted);'>No data</div>";
        return;
    }

    const values = items.map((item) => item.value);
    const maxValue = Math.max(...values, 0.001);

    const yMaxEl = document.getElementById(yMaxId);
    if (yMaxEl) {
        yMaxEl.textContent =
            state.historyMetric === "cost"
                ? formatHistoryCost(maxValue)
                : formatHistoryKwh(maxValue);
    }

    items.forEach((item, index) => {
        const ratio = item.value / maxValue;
        const heightPercent = Math.max(6, ratio * 100);

        const bar = document.createElement("button");
        bar.type = "button";
        bar.className = `history-bar ${fuel}`;
        bar.style.height = `${heightPercent}%`;

        const labelValue =
            state.historyMetric === "cost"
                ? formatHistoryCost(item.value)
                : formatHistoryKwh(item.value);

        bar.dataset.value = labelValue;
        bar.title = `${item.label}\n${labelValue}`;
        bar.setAttribute("aria-label", `${fuel} ${item.label} ${labelValue}`);

        bar.addEventListener("click", () => {
            const existing = chart.querySelector(".history-bar.selected");
            if (existing && existing !== bar) {
                existing.classList.remove("selected");
            }

            bar.classList.toggle("selected");
        });

        chart.appendChild(bar);

        const label = document.createElement("div");
        label.className = "history-time-label";
        label.textContent = item.shortLabel;
        label.style.left = `${((index + 0.5) / items.length) * 100}%`;
        axis.appendChild(label);
    });
}

async function fetchHistoryDay(isoDate) {
    const response = await fetch(`/api/history/day?date=${encodeURIComponent(isoDate)}`, {
        headers: { Accept: "application/json" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load history day: HTTP ${response.status}`);
    }

    return response.json();
}

async function fetchHistoryWeek(isoDate) {
    const response = await fetch(`/api/history/week?date=${encodeURIComponent(isoDate)}`, {
        headers: { Accept: "application/json" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load history week: HTTP ${response.status}`);
    }

    return response.json();
}

async function fetchHistoryMonth(isoDate) {
    const response = await fetch(`/api/history/month?date=${encodeURIComponent(isoDate)}`, {
        headers: { Accept: "application/json" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load history month: HTTP ${response.status}`);
    }

    return response.json();
}

function renderHistorySummary(electricityData, gasData) {
    const electricitySummary = document.getElementById("history-electricity-summary");
    const gasSummary = document.getElementById("history-gas-summary");

    if (electricitySummary) {
        electricitySummary.textContent = state.historyMetric === "cost" ? "Cost" : "kWh";
    }

    if (gasSummary) {
        gasSummary.textContent = state.historyMetric === "cost" ? "Cost" : "kWh";
    }

    const electricityTotal = document.getElementById("history-electricity-total");
    const electricityStanding = document.getElementById("history-electricity-standing");
    const gasTotal = document.getElementById("history-gas-total");
    const gasStanding = document.getElementById("history-gas-standing");

    if (electricityTotal) {
        electricityTotal.textContent =
            state.historyMetric === "cost"
                ? `Total ${formatGbp(electricityData?.total_cost_gbp)}`
                : `Total ${formatHistoryKwh(electricityData?.total_consumption_kwh)}`;
    }

    if (electricityStanding) {
        electricityStanding.textContent =
            state.historyMetric === "cost"
                ? `Standing ${formatGbp(electricityData?.standing_charge_gbp)}`
                : "";
    }

    if (gasTotal) {
        gasTotal.textContent =
            state.historyMetric === "cost"
                ? `Total ${formatGbp(gasData?.total_cost_gbp)}`
                : `Total ${formatHistoryKwh(gasData?.total_consumption_kwh)}`;
    }

    if (gasStanding) {
        gasStanding.textContent =
            state.historyMetric === "cost"
                ? `Standing ${formatGbp(gasData?.standing_charge_gbp)}`
                : "";
    }
}

function renderHistoryDay(data) {
    const dateLabel = document.getElementById("history-date-label");
    if (dateLabel && data.electricity?.date) {
        dateLabel.textContent = formatHistoryDateLabel(data.electricity.date);
    }

    renderHistorySummary(data.electricity, data.gas);

    renderHistoryChart(
        "history-electricity-chart",
        "history-electricity-axis",
        data.electricity?.slots ?? [],
        "electricity",
        "electricity-y-max",
    );

    renderHistoryChart(
        "history-gas-chart",
        "history-gas-axis",
        data.gas?.slots ?? [],
        "gas",
        "gas-y-max",
    );
}

function buildAggregateItems(days, mode) {
    return days.map((day) => {
        const date = parseIsoDateToLocalDate(day.date);

        let shortLabel = day.date;
        if (mode === "week") {
            shortLabel = date.toLocaleDateString("en-GB", { weekday: "short" });
        } else if (mode === "month") {
            shortLabel = date.getDate().toString();
        }

        return {
            label: formatHistoryDateLabel(day.date),
            shortLabel,
            value: getHistorySummaryValue(day),
        };
    });
}

function renderHistoryWeek(data) {
    const dateLabel = document.getElementById("history-date-label");
    if (dateLabel) {
        dateLabel.textContent = formatWeekRangeLabel(data.start_date, data.end_date);
    }

    renderHistorySummary(data.electricity, data.gas);

    renderHistoryAggregateChart(
        "history-electricity-chart",
        "history-electricity-axis",
        buildAggregateItems(data.electricity?.days ?? [], "week"),
        "electricity",
        "electricity-y-max",
    );

    renderHistoryAggregateChart(
        "history-gas-chart",
        "history-gas-axis",
        buildAggregateItems(data.gas?.days ?? [], "week"),
        "gas",
        "gas-y-max",
    );
}

function renderHistoryMonth(data) {
    const dateLabel = document.getElementById("history-date-label");
    if (dateLabel) {
        dateLabel.textContent = formatMonthRangeLabel(data.start_date, data.end_date);
    }

    renderHistorySummary(data.electricity, data.gas);

    renderHistoryAggregateChart(
        "history-electricity-chart",
        "history-electricity-axis",
        buildAggregateItems(data.electricity?.days ?? [], "month"),
        "electricity",
        "electricity-y-max",
    );

    renderHistoryAggregateChart(
        "history-gas-chart",
        "history-gas-axis",
        buildAggregateItems(data.gas?.days ?? [], "month"),
        "gas",
        "gas-y-max",
    );
}

export async function loadHistory() {
    const maxDate = getYesterdayIsoDate();

    if (state.historySelectedDate > maxDate) {
        state.historySelectedDate = maxDate;
    }

    const isoDate = state.historySelectedDate ?? getYesterdayIsoDate();

    if (state.historyRange === "day") {
        const data = await fetchHistoryDay(isoDate);
        renderHistoryDay(data);
        updateNavigationButtons();
        return;
    }

    if (state.historyRange === "week") {
        const data = await fetchHistoryWeek(isoDate);
        renderHistoryWeek(data);
        updateNavigationButtons();
        return;
    }

    if (state.historyRange === "month") {
        const data = await fetchHistoryMonth(isoDate);
        renderHistoryMonth(data);
        updateNavigationButtons();
        return;
    }

    console.warn("Unknown history range", state.historyRange);
}

export async function loadHistoryYesterday() {
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    state.historySelectedDate = formatDateForApi(yesterday);

    await loadHistory();
}

export function setupHistoryModal() {
    const historyButton = document.getElementById("history-button");
    const root = document.getElementById("history-modal-root");

    if (!historyButton || !root) return;

    historyButton.addEventListener("click", openHistoryModal);

    root.addEventListener("click", async (event) => {
        const target = event.target;

        if (!(target instanceof HTMLElement)) return;

        if (target.id === "history-close-button" || target.id === "history-backdrop") {
            closeHistoryModal();
            return;
        }

        if (target.id === "history-metric-cost") {
            state.historyMetric = "cost";
            syncMetricButtons();
            await loadHistory();
            return;
        }

        if (target.id === "history-metric-kwh") {
            state.historyMetric = "kwh";
            syncMetricButtons();
            await loadHistory();
            return;
        }

        if (target.id === "history-range-day") {
            state.historyRange = "day";
            syncRangeButtons();
            await loadHistory();
            return;
        }

        if (target.id === "history-range-week") {
            state.historyRange = "week";
            syncRangeButtons();
            await loadHistory();
            return;
        }

        if (target.id === "history-range-month") {
            state.historyRange = "month";
            syncRangeButtons();
            await loadHistory();
            return;
        }

        if (target.id === "history-prev-button") {
            if (state.historyRange === "day") {
                shiftSelectedDate(-1);
            } else if (state.historyRange === "week") {
                shiftSelectedDate(-7);
            } else if (state.historyRange === "month") {
                shiftSelectedMonth(-1);
            }

            await loadHistory();
            return;
        }

        if (target.id === "history-next-button") {
            if (state.historyRange === "day") {
                if (state.historySelectedDate < getYesterdayIsoDate()) {
                    shiftSelectedDate(1);
                    await loadHistory();
                }
                return;
            }

            if (state.historyRange === "week") {
                if (state.historySelectedDate < getYesterdayIsoDate()) {
                    shiftSelectedDate(7);

                    if (state.historySelectedDate > getYesterdayIsoDate()) {
                        state.historySelectedDate = getYesterdayIsoDate();
                    }

                    await loadHistory();
                }
                return;
            }

            if (state.historyRange === "month") {
                if (state.historySelectedDate < getYesterdayIsoDate()) {
                    shiftSelectedMonth(1);

                    if (state.historySelectedDate > getYesterdayIsoDate()) {
                        state.historySelectedDate = getYesterdayIsoDate();
                    }

                    await loadHistory();
                }
                return;
            }

            return;
        }
    });
}