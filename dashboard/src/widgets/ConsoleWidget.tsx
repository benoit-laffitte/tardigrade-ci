interface ConsoleWidgetProps {
  logs: string;
}

// Renders operator log output in a dedicated full-width dashboard panel.
export function ConsoleWidget({ logs }: Readonly<ConsoleWidgetProps>) {
  return (
    <section className="panel panel-full console reveal" style={{ ["--delay" as string]: "0.32s" }}>
      <h2>Journal de bord</h2>
      <pre aria-live="polite">{logs}</pre>
    </section>
  );
}
