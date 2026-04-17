export interface GridConfig {
  columns: number;
  rowHeight: number;
  gap: number;
  padding: number;
  blockWidth: number;
}

export interface GridCell {
  row: number;
  col: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

const DEFAULT_CONFIG: GridConfig = {
  columns: 3,
  rowHeight: 250,
  gap: 24,
  padding: 48,
  blockWidth: 600,
};

export class GridLayout {
  private config: GridConfig;

  constructor(config?: Partial<GridConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /** Calculate grid positions for N blocks */
  layout(blockCount: number): GridCell[] {
    const { columns, rowHeight, gap, padding, blockWidth } = this.config;
    const cells: GridCell[] = [];

    for (let i = 0; i < blockCount; i++) {
      const row = Math.floor(i / columns);
      const col = i % columns;
      cells.push({
        row,
        col,
        x: padding + col * (blockWidth + gap),
        y: padding + row * (rowHeight + gap),
        width: blockWidth,
        height: rowHeight,
      });
    }

    return cells;
  }

  /** Get the cell for a specific index */
  getCell(index: number): GridCell {
    const cells = this.layout(index + 1);
    return cells[index];
  }

  /** Get total grid dimensions */
  getTotalSize(blockCount: number): { width: number; height: number } {
    const { columns, rowHeight, gap, padding, blockWidth } = this.config;
    const rows = Math.ceil(blockCount / columns);
    return {
      width: padding * 2 + columns * blockWidth + (columns - 1) * gap,
      height: padding * 2 + rows * rowHeight + (rows - 1) * gap,
    };
  }

  /** Find which cell a point falls in */
  hitTestGrid(x: number, y: number, blockCount: number): number | null {
    const cells = this.layout(blockCount);
    for (let i = 0; i < cells.length; i++) {
      const cell = cells[i];
      if (x >= cell.x && x <= cell.x + cell.width &&
          y >= cell.y && y <= cell.y + cell.height) {
        return i;
      }
    }
    return null;
  }

  /** Auto-arrange blocks by applying grid positions */
  autoArrange(
    blocks: Array<{ id: string }>,
    applyPosition: (id: string, x: number, y: number, w: number, h: number) => void
  ): void {
    const cells = this.layout(blocks.length);
    for (let i = 0; i < blocks.length; i++) {
      const cell = cells[i];
      applyPosition(blocks[i].id, cell.x, cell.y, cell.width, cell.height);
    }
  }

  /** Get/set config */
  getConfig(): GridConfig { return { ...this.config }; }
  setConfig(config: Partial<GridConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /** Presets */
  static compact(): GridLayout {
    return new GridLayout({ columns: 4, rowHeight: 180, gap: 12, blockWidth: 400 });
  }

  static spacious(): GridLayout {
    return new GridLayout({ columns: 2, rowHeight: 350, gap: 48, blockWidth: 800 });
  }

  static single(): GridLayout {
    return new GridLayout({ columns: 1, rowHeight: 500, gap: 32, blockWidth: 1200 });
  }
}
