# ⚡ Home Energy Dashboard (IHD)

> Built for a dedicated always-on home display — fast, local-first, and
> energy-aware.

A lightweight, real-time home energy dashboard designed for a **5" 800×480
touchscreen**, combining:

- 🧠 Smart Octopus Agile pricing (dynamic window)
- 🔌 Live Home Assistant power usage
- ☀️ Solar generation monitoring
- 🔋 Battery state display (future live integration ready)
- 💷 Live electricity and gas cost tracking
- 📊 Historical energy usage (day / week / month)
- ⚙️ Persistent user settings (local file-based)

Built with:

- 🦀 Rust backend (data + API + persistence)
- 🌐 Simple HTML/CSS/JS frontend (touch-first display)



## 📸 Features

### ⚡ Live Dashboard

The main dashboard is designed for quick glanceable use on a small always-on
screen.

It includes:

- Real-time clock
- Hybrid **House Usage** panel:
  - Current house load in watts
  - Current cost per hour
  - Current Agile unit rate
  - Import / export / solar-covered state
- **Costs Today** panel:
  - Electricity cost today
  - Gas cost today
  - Combined total
  - Daily budget progress gauges
- **Solar** panel:
  - Current generation
  - Solar generation gauge
  - Export indicator when power is being exported to the grid
- **Battery State** panel:
  - Battery percentage display
  - kWh estimate
  - Battery fill visual
  - Placeholder/status handling until live battery data is available
- Appliance status / recommendations:
  - Washer
  - Dishwasher
  - Dryer
- Agile pricing chart:
  - Rolling pricing window
  - Colour-coded cheap / normal / expensive slots
  - Adaptive window size
- Auto-refresh every 10 seconds



## 📸 Dashboard Preview

![Home Energy Dashboard](docs/images/dashboard-main.png)



## 💷 Device Costs View

The dashboard includes a device cost modal showing the most expensive monitored
devices, ordered from highest cost to lowest cost.

This allows quick answers to questions such as:

- What is costing the most right now?
- What has cost the most today?
- What was most expensive yesterday?
- What has cost the most this month?

The modal supports the following ranges:

- **Current** — current cost rate by device
- **Today** — device costs accumulated today
- **Yesterday** — previous day device costs
- **Month** — current month device costs

Each view displays:

- Total cost for the selected range
- Top cost devices, sorted highest first
- Horizontal bars scaled relative to the most expensive item
- Cost and percentage contribution per device
- Separate colour treatments for different ranges

Zero-cost devices are hidden so that the list remains useful and readable.



## 📊 History View

The history view is powered by locally stored Octopus Energy data.

It supports:

- Day view (30-minute slots)
- Week view (daily totals)
- Month view (daily totals)
- Toggle between:
  - Cost (£)
  - kWh (future expansion)



## ⚙️ Settings

Current settings include:

- Adjustable Agile chart window:
  - 12 hours (24 slots)
  - 18 hours (36 slots)
  - 24 hours (48 slots)
- Persisted locally (`data/settings.json`)
- Applied instantly without restarting the backend

Planned settings include:

- Electricity daily budget
- Gas daily budget
- Battery capacity
- Display preferences



## 🏗️ Architecture

```text
Home Assistant  →  Rust Backend  →  Web UI
     (API)           (/api)        (Touch display)
                         ↓
                  Local storage
              (agile + history + settings)
```



## ⚙️ Setup

### 1. Clone the project

```bash
git clone https://github.com/madbeardman/ihd-replacement
cd ihd-replacement
```



### 2. Configure environment variables

Create a `.env` file:

```env
HOME_ASSISTANT_URL=http://homeassistant.local:8123
HOME_ASSISTANT_TOKEN=your_ha_long_lived_access_token_here

OCTOPUS_API_KEY=your_sk_live_api_key_lives_here
OCTOPUS_ELECTRICITY_MPAN=your_mpan_lives_here
OCTOPUS_ELECTRICITY_SERIAL=your_electricity_meter_serial_number_lives_here
OCTOPUS_ELECTRICITY_STANDING_CHARGE_P_PER_DAY=your_electricity_standing_charge_lives_here
OCTOPUS_GAS_MPRN=your_gas_meter_mprn_lives_here
OCTOPUS_GAS_SERIAL=your_gas_meter_serial_number_lives_here
OCTOPUS_GAS_UNIT_RATE_P_PER_KWH=your_gas_per_kilowatt_charge_lives_here
OCTOPUS_GAS_STANDING_CHARGE_P_PER_DAY=your_gas_standing_charge_lives_here
OCTOPUS_GAS_CORRECTION_FACTOR=1.02264
OCTOPUS_GAS_CALORIFIC_VALUE=39.1
```



## 🔑 Home Assistant Token

To connect the dashboard to Home Assistant, you’ll need a **Long-Lived Access
Token**.

You can create one from your Home Assistant profile:

1. Open Home Assistant
2. Go to **Profile** (bottom left corner)
3. Scroll to **Long-Lived Access Tokens**
4. Click **Create Token** and copy the value

Add this token to your `.env` file, replacing
`your_ha_long_lived_access_token_here` with the newly created token.

> ⚠️ **Security Note**
> 
> Your Home Assistant Long-Lived Access Token provides full access to your Home
> Assistant instance.
> 
> - Never share this token publicly - Do not commit it to GitHub or version
> control - Store it securely in your `.env` file only
> 
> If you believe your token has been exposed, revoke it immediately from your
> Home Assistant profile and generate a new one.



### 3. Run backend

```bash
cargo run
```



### 4. Open dashboard

```text
http://localhost:3000
```



## 🔄 Data Sources

| Purpose | Source |
|---|---|
| House load | Home Assistant house usage sensor |
| Grid import / export | Octopus Mini current demand |
| Solar generation | Home Assistant solar power sensor |
| Electricity cost today | Octopus Energy Home Assistant sensor |
| Gas cost today | Octopus Energy Home Assistant sensor |
| Device costs | Home Assistant top-cost template sensors |
| Appliances | Derived from appliance power sensors / thresholds |
| Agile pricing | Octopus Energy API |



## 💾 Data Storage

Stored locally under `/data`:

```text
data/
├── agile/       # Agile pricing (daily JSON files)
├── history/     # Electricity + gas history
└── settings.json
```



## 💾 Historical Usage Data

The dashboard supports historical energy usage (day, week, and month views),
powered by data from the Octopus Energy API.

By default, only recent data (for example yesterday) is fetched automatically.
To populate a longer history, you can run a manual backfill.



### 🔄 Backfilling History

You can fetch historical usage data using the included CLI tool:

```bash
cargo run --bin backfill_history -- 184
```

This will download **184 days (~6 months)** of historical data and store it
locally.



### 📁 Where Data Is Stored

All historical data is saved as JSON files under:

```text
data/history/
```

These files are then used by the dashboard to render:

- 📅 Daily usage (30-minute slots)
- 📊 Weekly summaries
- 📈 Monthly summaries



### 🖥️ Where to Run It

You have two options:

**Option A — Run on the IHD device (recommended)**

- Run the command directly on your Raspberry Pi / IHD
- Data is immediately available to the dashboard

**Option B — Run on another machine**

- Run locally, for example on a laptop or desktop
- Copy the resulting `data/history/` folder to the IHD device



### ⚠️ Notes

- Requires valid Octopus API credentials configured in your environment
- Backfilling large ranges may take a little time due to API limits
- Existing files will be reused where possible to avoid unnecessary re-fetching



### 🚀 Tip

A good starting point is:

```bash
cargo run --bin backfill_history -- 90
```

This gives you **3 months of history**, which is enough to make the history
charts immediately useful without long fetch times.



## 🔌 API Endpoints

| Endpoint | Description |
|---|---|
| `/api/dashboard` | Full dashboard state |
| `/api/agile` | Agile rolling window |
| `/api/settings` (GET/POST) | Load / update settings |
| `/api/history/day` | Day history |
| `/api/history/week` | Week history |
| `/api/history/month` | Month history |



## 🚀 Roadmap

- Appliance scheduling planner
  - Select an appliance
  - Set expected run duration
  - Find the cheapest available Agile window
  - Optionally compare “run now” vs “run later”
- Battery integration with real charge / discharge insight
- Battery-aware optimisation
- Cost forecasting for today / tomorrow
- User-configurable budgets
- Notifications / companion app
- Advanced caching and performance tuning
- Demo Mode
  - Generate consistent, realistic sample data
  - Support screenshots, testing, and UI development without live dependencies



## 💡 Philosophy

> “Cost efficiency is great — simplicity should always win.”

The dashboard is designed to answer three questions quickly:

- What is my house doing right now?
- Is it costing me money?
- Should I wait or run something now?
