# TODO

## Pending Coordination

- [ ] **Metrick traction KPI wiring** — n8n workflow `SX8YMfgkgnaj2Qs0` needs `localpush.traction` branch connected to `upsert_metrics_batch` with header `X-Metric-Source: localpush.traction`. Flagged by Aston. Schedule before Phase 1 closes. (Cross-domain: coordinate with Metrick.)

## Known Issues

- [ ] **UX: Enable checkbox confusing** — "I did not recognize it as a checkbox". Defer to Google Stitch for redesign.
- [ ] **Old production LocalPush.app conflicts** — Kill before dev testing: `pkill -f LocalPush || true`
- [ ] **Port 1420 may be held** — `lsof -ti:1420 | xargs kill -9`
