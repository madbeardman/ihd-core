import { loadDashboard, updateClock, advanceUsageRotation } from "./dashboard.js";
import { setupSettingsModal, loadSettingsModalPartial } from "./settings.js";
import { loadHistoryModalPartial, setupHistoryModal } from "./history.js";
import { loadCostUsageModalPartial, setupCostUsageModal } from "./costs.js";

async function init() {
    updateClock();

    await loadHistoryModalPartial();
    await loadSettingsModalPartial();
    await loadCostUsageModalPartial();

    setupHistoryModal();

    setupCostUsageModal();

    setupSettingsModal(async () => {
        await loadDashboard();
    });

    await loadDashboard();

    setInterval(updateClock, 1000);
    setInterval(loadDashboard, 10000);
    setInterval(advanceUsageRotation, 8000);
}

init();