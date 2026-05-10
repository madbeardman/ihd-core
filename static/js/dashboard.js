import { state } from "./state.js";
import { renderAgileChart } from "./agile.js";
import {
    clamp,
    formatClock,
    formatGbp,
    formatPrice,
    isDevMode,
} from "./utils.js";

const DAILY_ELECTRICITY_BUDGET_GBP = 2.0;
const DAILY_GAS_BUDGET_GBP = 5.0;
const BATTERY_MAX_KWH = 4.0;

function getHouseUsageColour(watts) {
    if (watts < 100) return "var(--usage-green-bright)";
    if (watts < 200) return "var(--usage-green-soft)";
    if (watts < 1000) return "var(--normal)";
    if (watts < 2000) return "var(--usage-orange)";
    return "var(--usage-red)";
}

function getSolarColor(_watts) {
    return "#16a34a";
}

function showPollIndicator() {
    const el = document.getElementById("poll-indicator");
    if (!el) return;
    el.style.display = "inline-block";
}

function pulsePollIndicator() {
    const el = document.getElementById("poll-indicator");
    if (!el) return;

    el.classList.remove("pulse");
    void el.offsetWidth;
    el.classList.add("pulse");

    setTimeout(() => {
        el.classList.remove("pulse");
    }, 1000);
}

function setPollIndicatorOk() {
    const el = document.getElementById("poll-indicator");
    if (!el) return;

    el.classList.remove("error");
}

function setPollIndicatorError() {
    const el = document.getElementById("poll-indicator");
    if (!el) return;

    el.classList.remove("pulse");
    el.classList.add("error");
}

function updateSolarExportIcon(octopusDemandW) {
    const icon = document.getElementById("solar-export-icon");
    const amount = document.getElementById("solar-export-amount");
    const value = document.getElementById("solar-export-value");

    const isExporting =
        typeof octopusDemandW === "number" && octopusDemandW < -5;

    if (icon) {
        icon.toggleAttribute("hidden", !isExporting);
    }

    if (amount) {
        amount.toggleAttribute("hidden", !isExporting);
    }

    if (value) {
        value.textContent = isExporting
            ? `${Math.round(Math.abs(octopusDemandW))}W`
            : "--";
    }
}

function updateBudgetGauge(gaugeId, percentId, cost, budget) {
    const gauge = document.getElementById(gaugeId);
    const percentEl = document.getElementById(percentId);

    if (!gauge || !percentEl || typeof cost !== "number") return;

    const ratio = budget > 0 ? cost / budget : 0;
    const percentage = Math.min(Math.max(ratio * 100, 0), 100);

    gauge.setAttribute(
        "stroke-dasharray",
        `${Math.max(percentage, cost > 0 ? 4 : 0)} 100`,
    );

    percentEl.textContent = `${Math.round(percentage)}%`;
}

function updateCostsTodayPanel(metrics) {
    const totalEl = document.getElementById("costs-today-total");
    const electricityEl = document.getElementById("costs-today-electricity");
    const gasEl = document.getElementById("costs-today-gas");

    const electricity =
        typeof metrics?.cost_today_gbp === "number"
            ? metrics.cost_today_gbp
            : null;

    const gas =
        typeof metrics?.gas_cost_today_gbp === "number"
            ? metrics.gas_cost_today_gbp
            : null;

    const total =
        (typeof electricity === "number" ? electricity : 0) +
        (typeof gas === "number" ? gas : 0);

    if (totalEl) {
        totalEl.textContent =
            typeof electricity === "number" || typeof gas === "number"
                ? formatGbp(total)
                : "--";
    }

    if (electricityEl) {
        electricityEl.textContent =
            typeof electricity === "number" ? formatGbp(electricity) : "--";
    }

    if (gasEl) {
        gasEl.textContent =
            typeof gas === "number" ? formatGbp(gas) : "--";
    }

    updateBudgetGauge(
        "costs-electricity-gauge",
        "costs-electricity-percent",
        electricity,
        DAILY_ELECTRICITY_BUDGET_GBP,
    );

    updateBudgetGauge(
        "costs-gas-gauge",
        "costs-gas-percent",
        gas,
        DAILY_GAS_BUDGET_GBP,
    );

    document.getElementById("costs-electricity-budget").textContent =
        `Budget ${formatGbp(DAILY_ELECTRICITY_BUDGET_GBP)}`;

    document.getElementById("costs-gas-budget").textContent =
        `Budget ${formatGbp(DAILY_GAS_BUDGET_GBP)}`;
}

function renderUsageRotation() {
    if (!state.latestUsageMetrics) return;

    const valueEl = document.getElementById("usage-main-value");
    const subtextEl = document.getElementById("usage-subtext");

    if (!valueEl || !subtextEl) return;

    const states = [
        {
            value:
                typeof state.latestUsageMetrics.current_power_w === "number"
                    ? `${Math.round(state.latestUsageMetrics.current_power_w)}W`
                    : "--",
            subtext: "Electricity",
        },
        {
            value:
                typeof state.latestUsageMetrics.current_cost_per_hour_gbp === "number"
                    ? `${formatGbp(state.latestUsageMetrics.current_cost_per_hour_gbp)}/hr`
                    : "--",
            subtext:
                typeof state.latestUsageMetrics.current_price_p_per_kwh === "number"
                    ? `At ${formatPrice(state.latestUsageMetrics.current_price_p_per_kwh)}`
                    : "Current cost",
        },
    ];

    const current = states[state.usageRotationIndex % states.length];
    valueEl.textContent = current.value;
    subtextEl.textContent = current.subtext;
}

export function advanceUsageRotation() {
    if (!state.latestUsageMetrics) return;
    state.usageRotationIndex = (state.usageRotationIndex + 1) % 2;
    renderUsageRotation();
}

export function updateClock() {
    const clock = document.getElementById("header-time");
    if (!clock) return;
    clock.textContent = formatClock();
}

function updateBatteryPanel(battery) {
    const percentEl = document.getElementById("battery-percentage");
    const kwhEl = document.getElementById("battery-kwh");
    const fillEl = document.getElementById("battery-fill");
    const statusEl = document.getElementById("battery-status");

    if (!percentEl || !kwhEl || !fillEl || !statusEl) return;

    if (!battery || typeof battery.soc !== "number") {
        percentEl.textContent = "--";
        kwhEl.textContent = "--";
        statusEl.textContent = "Unavailable";
        fillEl.style.height = "0%";
        return;
    }

    const soc = Math.round(battery.soc);
    const kwh = (soc / 100) * BATTERY_MAX_KWH;

    percentEl.textContent = `${soc}%`;
    kwhEl.textContent = `${kwh.toFixed(1)}kWh`;

    fillEl.style.height = `${soc}%`;

    // status logic
    if (battery.power_w > 50) {
        statusEl.textContent = "Charging";
        statusEl.style.color = "var(--cheap)";
    } else if (battery.power_w < -50) {
        statusEl.textContent = "Discharging";
        statusEl.style.color = "var(--usage-orange)";
    } else {
        statusEl.textContent = "Idle";
        statusEl.style.color = "var(--muted)";
    }
}

function updateHouseUsageGauge(watts) {
    const gaugeArc = document.getElementById("usage-gauge-electric");

    if (!gaugeArc || typeof watts !== "number") return;

    const maxWatts = 4000;
    const clampedWatts = clamp(watts, 0, maxWatts);
    const colour = getHouseUsageColour(clampedWatts);
    gaugeArc.style.opacity = "1";

    if (clampedWatts <= 0) {
        gaugeArc.setAttribute("stroke-dasharray", "1 100"); // Show a small sliver to indicate 0 usage
        gaugeArc.style.stroke = "var(--usage-green-soft)";
        return;
    }

    const linearRatio = clampedWatts / maxWatts;
    const percentage = Math.max(Math.sqrt(linearRatio) * 100, 10);

    gaugeArc.setAttribute("stroke-dasharray", `${percentage} 100`);
    gaugeArc.style.stroke = colour;
}

function updateSolarGauge(watts) {
    const gaugeArc = document.getElementById("solar-gauge-fill");
    const gaugeTrack = document.getElementById("solar-gauge-track");

    if (!gaugeArc || !gaugeTrack || typeof watts !== "number") return;

    const maxWatts = 3480;
    const clampedWatts = clamp(watts, 0, maxWatts);

    const linearRatio = clampedWatts / maxWatts;
    let percentage = Math.sqrt(linearRatio) * 100;

    if (clampedWatts > 0) {
        percentage = Math.max(percentage, 8);
    } else {
        percentage = 0;
    }

    const colour = getSolarColor(clampedWatts);

    gaugeArc.setAttribute("stroke-dasharray", `${percentage} 100`);
    gaugeArc.style.stroke = colour;
    gaugeTrack.style.stroke = "";
}

function updateApplianceRow(appliances) {
    const washerEl = document.getElementById("appliance-washing-machine");
    const dishwasherEl = document.getElementById("appliance-dishwasher");
    const dryerEl = document.getElementById("appliance-tumble-dryer");

    if (isDevMode()) {
        console.log("Updating appliances:", appliances);
    }

    if (!washerEl || !dishwasherEl || !dryerEl || !appliances) return;

    const washer = appliances.washing_machine?.display ?? "--";
    const dishwasher = appliances.dishwasher?.display ?? "--";
    const dryer = appliances.tumble_dryer?.display ?? "--";

    if (isDevMode()) {
        console.log("Appliance display values:", { washer, dishwasher, dryer });
    }

    washerEl.textContent = washer;
    dishwasherEl.textContent = dishwasher;
    dryerEl.textContent = dryer;

    washerEl.classList.toggle("running", appliances.washing_machine?.running === true);
    dishwasherEl.classList.toggle("running", appliances.dishwasher?.running === true);
    dryerEl.classList.toggle("running", appliances.tumble_dryer?.running === true);
}

function updateHouseUsagePanel(metrics, live) {
    const loadEl = document.getElementById("usage-load-value");
    const costEl = document.getElementById("usage-cost-value");
    const rateEl = document.getElementById("usage-cost-rate");

    if (!metrics || !loadEl || !costEl || !rateEl) return;

    const watts = metrics.current_power_w ?? 0;
    const costPerHour = metrics.current_cost_per_hour_gbp ?? 0;
    const price = metrics.current_price_p_per_kwh ?? null;
    const demand = live?.octopus_current_demand_w ?? 0;

    // --- Main values ---
    loadEl.textContent = `${Math.round(Math.abs(watts))}W`;
    costEl.textContent = `${formatGbp(costPerHour)}/hr`;
    rateEl.textContent =
        typeof price === "number" ? `At ${formatPrice(price)}` : "--";
}

export async function loadDashboard() {
    if (state.dashboardRequestInFlight) return;
    state.dashboardRequestInFlight = true;

    const output = document.getElementById("output");

    try {
        const response = await fetch("/api/dashboard", {
            headers: { Accept: "application/json" },
            cache: "no-store",
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        const data = await response.json();

        showPollIndicator();
        setPollIndicatorOk();
        pulsePollIndicator();

        const indicator = document.getElementById("poll-indicator");
        if (indicator) {
            indicator.style.display = "inline-block";
        }

        if (output) {
            output.textContent = JSON.stringify(data, null, 2);
        }

        updateHouseUsagePanel(data.usage_metrics, data.live);
        updateCostsTodayPanel(data.usage_metrics);
        updateBatteryPanel(data.battery);

        const housePower =
            typeof data.usage_metrics?.current_power_w === "number"
                ? data.usage_metrics.current_power_w
                : 0;

        updateHouseUsageGauge(Math.round(housePower));

        if (typeof data.live?.solar_generation_w === "number") {
            let solar = Math.round(data.live.solar_generation_w);

            if (solar < 10) {
                solar = 0;
            }

            const valueEl = document.querySelector("#solar-panel .panel-value");
            if (valueEl) valueEl.textContent = `${solar}W`;

            updateSolarGauge(solar);
        } else {
            const valueEl = document.querySelector("#solar-panel .panel-value");
            if (valueEl) valueEl.textContent = "--";

            updateSolarGauge(0);
        }

        updateSolarExportIcon(data.live?.octopus_current_demand_w);

        renderAgileChart(data.agile);
        updateApplianceRow(data.appliances);
    } catch (error) {
        const dashboard = document.getElementById("dashboard");
        const devMode = dashboard?.dataset.devMode === "true";

        const updatedEl = document.getElementById("last-updated");
        if (updatedEl) {
            updatedEl.textContent = devMode ? "Update failed" : "";
        }

        updateSolarExportIcon(0);

        showPollIndicator();
        setPollIndicatorError();

        if (output) {
            output.textContent = String(error);
        }
    } finally {
        state.dashboardRequestInFlight = false;
    }
}