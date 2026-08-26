const paths = {
  logo: '<path d="M6 5v14M18 5l-8 7 8 7"/><path d="M6 12h4"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  settings:
    '<path d="M9 4h6l1 3 3 1 1 5-3 2-1 4H9l-1-3-3-1-1-5 3-2z"/><circle cx="12" cy="12" r="3"/>',
  chevron: '<path d="m8 10 4 4 4-4"/>',
  arrow: '<path d="M5 12h14m-5-5 5 5-5 5"/>',
  input: '<path d="M12 4v16m-5-5 5 5 5-5"/>',
  output: '<path d="M12 20V4m-5 5 5-5 5 5"/>',
  overview:
    '<rect x="4" y="4" width="6" height="6" rx="1.5"/><rect x="14" y="4" width="6" height="6" rx="1.5"/><rect x="4" y="14" width="6" height="6" rx="1.5"/><rect x="14" y="14" width="6" height="6" rx="1.5"/>',
  cost: '<path d="M12 3v18m5-14H9a3 3 0 0 0 0 6h6a3 3 0 0 1 0 6H6"/>',
  latency: '<circle cx="12" cy="13" r="8"/><path d="M12 9v4l3 2M9 2h6"/>',
  distribution: '<path d="M4 5h16M4 12h11M4 19h7"/>',
  accounts:
    '<rect x="3" y="5" width="18" height="15" rx="3"/><circle cx="9" cy="11" r="2"/><path d="M6 17c0-3 6-3 6 0m3-6h3m-3 4h3"/>',
  refresh:
    '<path d="M20 10a8 8 0 0 0-14-5L3 8m0-5v5h5m-4 6a8 8 0 0 0 14 5l3-3m0 5v-5h-5"/>',
  key: '<circle cx="8" cy="8" r="4"/><path d="m11 11 9 9m-3-3 3-3m-6 0 3-3"/>',
  shield:
    '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6z"/><path d="m8 12 3 3 5-6"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10v1"/>',
  link: '<path d="m9 15 6-6m-6 2L7 9a4 4 0 0 1 6-6l3 3m-1 7 2 2a4 4 0 0 1-6 6l-3-3"/>',
};
export const icon = (name) =>
  name === "logo"
    ? '<img src="/keeper.svg" alt="Keeper" width="38" height="28">'
    : `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || paths.info}</svg>`;
