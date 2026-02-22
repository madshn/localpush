# TODO

## Known Issues

- [ ] **UX: Enable checkbox confusing** — "I did not recognize it as a checkbox". Defer to Google Stitch for redesign.
- [ ] **Old production LocalPush.app conflicts** — Kill before dev testing: `pkill -f LocalPush || true`
- [ ] **Port 1420 may be held** — `lsof -ti:1420 | xargs kill -9`
