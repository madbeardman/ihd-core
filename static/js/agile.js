import { formatHourLabel, formatPrice } from "./utils.js";

export function renderAgileChart(data) {
    const chart = document.getElementById("agile-chart");
    const timeAxis = document.getElementById("agile-time-axis");

    if (!chart || !timeAxis) return;

    timeAxis.innerHTML = "";
    chart.innerHTML = "";

    if (!data?.slots || data.slots.length === 0) {
        chart.innerHTML = "<div style='color: var(--muted);'>No data</div>";
        return;
    }

    const values = data.slots.map((slot) => slot.value_inc_vat);
    const hasNegative = values.some((v) => v < 0);
    const minBarHeight = 12;

    chart.className = hasNegative ? "agile-chart mixed" : "agile-chart positive-only";

    if (!hasNegative) {
        const maxPrice = Math.max(...values, 0.001);

        for (const slot of data.slots) {
            const bar = document.createElement("div");
            const heightPercent = (slot.value_inc_vat / maxPrice) * 85;
            const finalHeight = Math.max(slot.value_inc_vat > 0 ? minBarHeight : 0, heightPercent);

            bar.className = `agile-bar ${slot.band}${slot.is_now ? " now" : ""}`;
            bar.style.height = `${finalHeight}%`;
            bar.dataset.price = formatPrice(slot.value_inc_vat);
            bar.title =
                `${slot.source_day} slot ${slot.source_index}\n` +
                `${formatPrice(slot.value_inc_vat)}\n` +
                `${slot.valid_from} → ${slot.valid_to}` +
                (slot.is_now ? "\nCURRENT SLOT" : "");

            chart.appendChild(bar);
        }

        const bars = Array.from(chart.children);
        const chartRect = chart.getBoundingClientRect();

        data.slots.forEach((slot, index) => {
            const date = new Date(slot.valid_from);

            if (date.getMinutes() === 0 && bars[index]) {
                const barRect = bars[index].getBoundingClientRect();
                const label = document.createElement("div");

                label.className = "agile-time-label";
                label.textContent = formatHourLabel(slot.valid_from);

                const barCenter = barRect.left - chartRect.left + barRect.width / 2;
                label.style.left = `${barCenter}px`;

                timeAxis.appendChild(label);
            }
        });

        return;
    }

    const maxPositive = Math.max(...values.filter((v) => v > 0), 0);
    const maxNegativeAbs = Math.max(
        ...values.filter((v) => v < 0).map((v) => Math.abs(v)),
        0,
    );
    const scaleMax = Math.max(maxPositive, maxNegativeAbs, 0.001);

    for (const slot of data.slots) {
        const wrap = document.createElement("div");
        wrap.className = "agile-bar-wrap";

        const bar = document.createElement("div");
        const value = slot.value_inc_vat;
        const heightPercent = (Math.abs(value) / scaleMax) * 50;
        const finalHeight = Math.max(Math.abs(value) > 0 ? minBarHeight : 0, heightPercent);

        bar.className = `agile-bar ${slot.band}${slot.is_now ? " now" : ""} ${value < 0 ? "negative" : "positive"
            }`;
        bar.style.height = `${finalHeight}%`;
        bar.dataset.price = formatPrice(slot.value_inc_vat);
        bar.title =
            `${slot.source_day} slot ${slot.source_index}\n` +
            `${formatPrice(slot.value_inc_vat)}\n` +
            `${slot.valid_from} → ${slot.valid_to}` +
            (slot.is_now ? "\nCURRENT SLOT" : "");

        wrap.appendChild(bar);
        chart.appendChild(wrap);
    }

    const wraps = Array.from(chart.querySelectorAll(".agile-bar-wrap"));
    const chartRect = chart.getBoundingClientRect();

    data.slots.forEach((slot, index) => {
        const date = new Date(slot.valid_from);

        if (date.getMinutes() === 0 && wraps[index]) {
            const wrapRect = wraps[index].getBoundingClientRect();
            const label = document.createElement("div");

            label.className = "agile-time-label";
            label.textContent = formatHourLabel(slot.valid_from);

            const center = wrapRect.left - chartRect.left + wrapRect.width / 2;
            label.style.left = `${center}px`;

            timeAxis.appendChild(label);
        }
    });
}