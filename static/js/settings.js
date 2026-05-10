async function fetchSettings() {
    const response = await fetch("/api/settings", {
        headers: { Accept: "application/json" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load settings: HTTP ${response.status}`);
    }

    return response.json();
}

async function saveSettings() {
    const active = document.querySelector(
        "#settings-agile-window-slots .settings-segment.active",
    );

    if (!active) return false;

    const value = Number(active.dataset.value);

    const response = await fetch("/api/settings", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
        },
        body: JSON.stringify({
            agile_window_slots: value,
        }),
    });

    if (!response.ok) {
        throw new Error(`Failed to save settings: HTTP ${response.status}`);
    }

    return true;
}

export async function loadSettingsModalPartial() {
    const root = document.getElementById("settings-modal-root");
    if (!root) return;

    const response = await fetch("/static/partials/settings-modal.html", {
        headers: { Accept: "text/html" },
        cache: "no-store",
    });

    if (!response.ok) {
        throw new Error(`Failed to load settings modal partial: HTTP ${response.status}`);
    }

    root.innerHTML = await response.text();
}

function setSegmentValue(containerId, value) {
    const container = document.getElementById(containerId);
    if (!container) return;

    container.querySelectorAll(".settings-segment").forEach((btn) => {
        btn.classList.toggle("active", Number(btn.dataset.value) === value);
    });
}

async function populateSettingsForm() {
    const settings = await fetchSettings();
    setSegmentValue("settings-agile-window-slots", settings.agile_window_slots ?? 24);
}

export async function openSettingsModal() {
    const modal = document.getElementById("settings-modal");
    const backdrop = document.getElementById("settings-backdrop");

    if (!modal || !backdrop) return;

    await populateSettingsForm();

    modal.removeAttribute("hidden");
    backdrop.removeAttribute("hidden");
}

export function closeSettingsModal() {
    const modal = document.getElementById("settings-modal");
    const backdrop = document.getElementById("settings-backdrop");

    if (!modal || !backdrop) return;

    modal.setAttribute("hidden", "");
    backdrop.setAttribute("hidden", "");
}

export async function setupSettingsModal(onSettingsSaved) {
    const root = document.getElementById("settings-modal-root");
    const openButton = document.getElementById("settings-button");

    if (!root || !openButton) return;

    openButton.addEventListener("click", async () => {
        await openSettingsModal();
    });

    root.addEventListener("click", async (event) => {
        const target = event.target;

        if (!(target instanceof HTMLElement)) return;

        if (target.id === "settings-close-button" || target.id === "settings-backdrop") {
            closeSettingsModal();
            return;
        }

        if (target.classList.contains("settings-segment")) {
            const container = target.parentElement;
            if (!container) return;

            container.querySelectorAll(".settings-segment").forEach((btn) => {
                btn.classList.remove("active");
            });

            target.classList.add("active");
            return;
        }

        if (target.id === "settings-save-button") {
            await saveSettings();

            if (typeof onSettingsSaved === "function") {
                await onSettingsSaved();
            }

            closeSettingsModal();
        }
    });

    document.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
            closeSettingsModal();
        }
    });
}