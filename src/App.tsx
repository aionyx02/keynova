import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [name, setName] = useState("Keynova");
  const [pingReply, setPingReply] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  async function pingRust() {
    setIsLoading(true);
    try {
      const message = await invoke<string>("cmd_ping", { name });
      setPingReply(message);
    } catch (error) {
      setPingReply(`IPC call failed: ${String(error)}`);
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <main className="min-h-screen bg-slate-100 p-6 text-slate-900">
      <section className="mx-auto mt-14 w-full max-w-xl rounded-2xl bg-white p-7 shadow-sm ring-1 ring-slate-200">
        <h1 className="text-2xl font-bold">Keynova Phase 0 IPC Check</h1>
        <p className="mt-2 text-sm text-slate-600">
          前端透過 Tauri IPC 呼叫 Rust 指令，確認基本通訊鏈路。
        </p>

        <form
          className="mt-6 flex flex-col gap-3 sm:flex-row"
          onSubmit={(event) => {
            event.preventDefault();
            void pingRust();
          }}
        >
          <input
            className="w-full rounded-lg border border-slate-300 px-3 py-2 outline-none transition focus:border-slate-500"
            value={name}
            onChange={(event) => setName(event.currentTarget.value)}
            placeholder="Enter a name"
          />
          <button
            type="submit"
            className="rounded-lg bg-slate-900 px-4 py-2 font-medium text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:bg-slate-400"
            disabled={isLoading}
          >
            {isLoading ? "Pinging..." : "Ping Rust"}
          </button>
        </form>

        <div className="mt-5 rounded-lg bg-slate-50 px-4 py-3 text-sm text-slate-700 ring-1 ring-slate-200">
          {pingReply || "等待執行 IPC 呼叫"}
        </div>
      </section>
    </main>
  );
}

export default App;
