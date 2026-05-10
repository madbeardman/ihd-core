function formatCostValue(value) {
    if (typeof value !== "number" || Number.isNaN(value)) {
        return "£0.00";
    }

    if (value === 0) {
        return "£0.00";
    }

    if (value < 0.01) {
        return `£${value.toFixed(3)}`;
    }

    return `£${value.toFixed(2)}`;
}

async function fetchDeviceCosts() {
    const response = await fetch("/api/dashboard", {
        headers: { Accept: "application/json" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load device costs: HTTP ${response.status}`);
    }

    return response.json();
}

function getCostItems(data, range) {
    const items = data?.live?.device_costs?.[range]?.items ?? [];

    return items
        .filter((item) => typeof item.cost_gbp === "number" && item.cost_gbp > 0)
        .sort((a, b) => b.cost_gbp - a.cost_gbp);
}

function getRangeLabel(range) {
    switch (range) {
        case "current":
            return "Current";
        case "yesterday":
            return "Yesterday";
        case "month":
            return "Month";
        case "today":
        default:
            return "Today";
    }
}

function renderCostUsageList(items, mode) {
    const list = document.getElementById("cost-usage-list");
    const totalEl = document.getElementById("cost-usage-total");

    if (!list) return;

    list.innerHTML = "";

    if (!items.length) {
        if (totalEl) {
            totalEl.textContent = "Total £0.00";
        }

        list.innerHTML = `<div class="cost-usage-empty">No cost data available</div>`;
        return;
    }

    const total = items.reduce((sum, item) => sum + item.cost_gbp, 0);

    if (totalEl) {
        totalEl.textContent = `Total ${formatCostValue(total)}`;
    }

    const maxValue = Math.max(...items.map((item) => item.cost_gbp), 0.001);

    for (const item of items) {
        const row = document.createElement("div");
        row.className = "cost-usage-row";

        if (item.cost_gbp < maxValue * 0.15) {
            row.dataset.low = "true";
        }

        const header = document.createElement("div");
        header.className = "cost-usage-row-header";

        const name = document.createElement("div");
        name.className = "cost-usage-name";
        name.textContent = item.name;

        const value = document.createElement("div");
        value.className = "cost-usage-value";
        const percentage = (item.cost_gbp / total) * 100;

        value.textContent = `${formatCostValue(item.cost_gbp)} (${percentage.toFixed(0)}%)`;

        header.appendChild(name);
        header.appendChild(value);

        const barTrack = document.createElement("div");
        barTrack.className = "cost-usage-bar-track";

        const barFill = document.createElement("div");
        barFill.className = `cost-usage-bar-fill cost-usage-bar-${mode}`;
        barFill.style.width = "0%";

        requestAnimationFrame(() => {
            barFill.style.width = `${Math.max((item.cost_gbp / maxValue) * 100, 4)}%`;
        });

        barTrack.appendChild(barFill);
        row.appendChild(header);
        row.appendChild(barTrack);
        list.appendChild(row);
    }
}

function syncRangeButtons(range) {
    document.getElementById("cost-usage-range-today")?.classList.toggle(
        "active",
        range === "today",
    );
    document.getElementById("cost-usage-range-yesterday")?.classList.toggle(
        "active",
        range === "yesterday",
    );
    document.getElementById("cost-usage-range-month")?.classList.toggle(
        "active",
        range === "month",
    );
    document.getElementById("cost-usage-range-current")?.classList.toggle(
        "active",
        range === "current",
    );

    const subtitle = document.getElementById("cost-usage-subtitle");
    if (subtitle) {
        subtitle.textContent = getRangeLabel(range);
    }
}

export async function loadCostUsageModalPartial() {
    const root = document.getElementById("cost-usage-modal-root");
    if (!root) return;

    const response = await fetch("/static/partials/cost-usage-modal.html", {
        headers: { Accept: "text/html" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load cost usage modal partial: HTTP ${response.status}`);
    }

    root.innerHTML = await response.text();
}

export async function loadCostUsage(range = "current") {
    const data = await fetchDeviceCosts();
    const items = getCostItems(data, range);

    syncRangeButtons(range);
    renderCostUsageList(items, range);
}

export async function openCostUsageModal(range = "current") {
    const modal = document.getElementById("cost-usage-modal");
    const backdrop = document.getElementById("cost-usage-backdrop");

    if (!modal || !backdrop) return;

    modal.removeAttribute("hidden");
    backdrop.removeAttribute("hidden");

    await loadCostUsage(range);
}

export function closeCostUsageModal() {
    const modal = document.getElementById("cost-usage-modal");
    const backdrop = document.getElementById("cost-usage-backdrop");

    if (!modal || !backdrop) return;

    modal.setAttribute("hidden", "");
    backdrop.setAttribute("hidden", "");
}

export function setupCostUsageModal() {
    const openButton = document.getElementById("costs-button");
    const root = document.getElementById("cost-usage-modal-root");

    if (!openButton || !root) return;

    let selectedRange = "current";

    openButton.addEventListener("click", async () => {
        await openCostUsageModal(selectedRange);
    });

    root.addEventListener("click", async (event) => {
        const target = event.target;

        if (!(target instanceof HTMLElement)) return;

        if (
            target.id === "cost-usage-close-button" ||
            target.id === "cost-usage-backdrop"
        ) {
            closeCostUsageModal();
            return;
        }

        if (target.id === "cost-usage-range-today") {
            selectedRange = "today";
            await loadCostUsage(selectedRange);
            return;
        }

        if (target.id === "cost-usage-range-yesterday") {
            selectedRange = "yesterday";
            await loadCostUsage(selectedRange);
            return;
        }

        if (target.id === "cost-usage-range-month") {
            selectedRange = "month";
            await loadCostUsage(selectedRange);
            return;
        }

        if (target.id === "cost-usage-range-current") {
            selectedRange = "current";
            await loadCostUsage(selectedRange);
        }
    });

    document.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
            closeCostUsageModal();
        }
    });
}