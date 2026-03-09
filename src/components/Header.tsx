import { Show } from "solid-js";
import { activeAccount } from "../stores/cloud";
import { scanning, startScan } from "../stores/scan";
import { toggleTheme, theme } from "../stores/theme";

interface Props {
  searchQuery: string;
  onSearch: (query: string) => void;
}

export default function Header(props: Props) {
  const account = activeAccount;

  const handleScan = () => {
    const acc = account();
    if (acc) {
      startScan(acc.id);
    }
  };

  return (
    <header class="header">
      <input
        id="search-input"
        class="header-search"
        type="text"
        placeholder="Search resources... (/)"
        value={props.searchQuery}
        onInput={(e) => props.onSearch(e.currentTarget.value)}
      />

      <div class="header-actions">
        <Show when={account()}>
          <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>
            {account()!.provider.toUpperCase()}: {account()!.display_name}
          </span>
        </Show>

        <button
          class="btn btn-primary btn-sm"
          onClick={handleScan}
          disabled={scanning() || !account()}
        >
          {scanning() ? "Scanning..." : "Scan"}
        </button>

        <button class="btn btn-sm" onClick={toggleTheme}>
          {theme() === "dark" ? "Light" : "Dark"}
        </button>
      </div>
    </header>
  );
}
