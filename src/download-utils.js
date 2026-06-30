import { EXPORT_COLUMNS, EXPORT_FILENAME_PREFIX } from "./export-config.js";

export function buildExportFilename(format, now = new Date()) {
  const timestamp = now.toISOString().replace(/[:.]/g, "-");
  return `${EXPORT_FILENAME_PREFIX}-${timestamp}.${format}`;
}

export function exportRowsToText(rows, format) {
  if (format === "json") {
    return {
      content: JSON.stringify(rows, null, 2),
      mimeType: "application/json",
    };
  }

  return {
    content: rowsToCsv(rows),
    mimeType: "text/csv",
  };
}

function rowsToCsv(rows) {
  const lines = [EXPORT_COLUMNS.join(",")];
  for (const row of rows) {
    lines.push(EXPORT_COLUMNS.map((col) => csvCell(row[col])).join(","));
  }
  return lines.join("\n") + "\n";
}

function csvCell(value) {
  if (value === null || value === undefined) return "";
  const text = String(value);
  if (/[",\n\r]/.test(text)) return `"${text.replace(/"/g, '""')}"`;
  return text;
}

export function downloadTextFile(filename, content, mimeType) {
  const blob = new Blob([content], { type: `${mimeType};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");

  link.href = url;
  link.download = filename;
  link.style.display = "none";

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
