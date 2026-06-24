import type { Groups } from "tauri-plugin-mihomo-api";
import "./App.css";

function App() {
  const example: Groups = { proxies: [] };

  return (
    <main className="container">
      <h1>Tauri Plugin Mihomo API</h1>
      <p>The frontend package now exports generated Mihomo types only.</p>
      <pre>{JSON.stringify(example, null, 2)}</pre>
    </main>
  );
}

export default App;
