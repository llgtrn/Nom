export interface BlockNode {
    id: string;
    parentId: string | null;
    children: string[];
    type: string;
    acceptsChildren: boolean;
    maxChildren: number; // -1 for unlimited
}

export class BlockHierarchy {
    private nodes: Map<string, BlockNode> = new Map();
    private listeners: (() => void)[] = [];

    /** Register a block */
    register(id: string, type: string, acceptsChildren: boolean, maxChildren = -1): void {
        this.nodes.set(id, {
            id, parentId: null, children: [], type, acceptsChildren, maxChildren,
        });
        this.notify();
    }

    /** Remove a block and reparent its children to its parent */
    remove(id: string): void {
        const node = this.nodes.get(id);
        if (!node) return;
        // Reparent children
        for (const childId of node.children) {
            const child = this.nodes.get(childId);
            if (child) child.parentId = node.parentId;
            if (node.parentId) {
                const parent = this.nodes.get(node.parentId);
                if (parent) parent.children.push(childId);
            }
        }
        // Remove from parent
        if (node.parentId) {
            const parent = this.nodes.get(node.parentId);
            if (parent) parent.children = parent.children.filter(c => c !== id);
        }
        this.nodes.delete(id);
        this.notify();
    }

    /** Move a block to be a child of another block */
    setParent(childId: string, parentId: string | null): boolean {
        const child = this.nodes.get(childId);
        if (!child) return false;

        if (parentId) {
            const parent = this.nodes.get(parentId);
            if (!parent) return false;
            if (!parent.acceptsChildren) return false;
            if (parent.maxChildren >= 0 && parent.children.length >= parent.maxChildren) return false;
            if (this.isAncestor(parentId, childId)) return false; // prevent cycles
        }

        // Remove from old parent
        if (child.parentId) {
            const oldParent = this.nodes.get(child.parentId);
            if (oldParent) oldParent.children = oldParent.children.filter(c => c !== childId);
        }

        // Add to new parent
        child.parentId = parentId;
        if (parentId) {
            const parent = this.nodes.get(parentId)!;
            parent.children.push(childId);
        }

        this.notify();
        return true;
    }

    /** Get root blocks (no parent) */
    getRoots(): BlockNode[] {
        return Array.from(this.nodes.values()).filter(n => n.parentId === null);
    }

    /** Get children of a block */
    getChildren(id: string): BlockNode[] {
        const node = this.nodes.get(id);
        if (!node) return [];
        return node.children.map(cid => this.nodes.get(cid)).filter(Boolean) as BlockNode[];
    }

    /** Get full subtree (depth-first) */
    getSubtree(id: string): BlockNode[] {
        const result: BlockNode[] = [];
        const visit = (nodeId: string) => {
            const node = this.nodes.get(nodeId);
            if (!node) return;
            result.push(node);
            for (const childId of node.children) visit(childId);
        };
        visit(id);
        return result;
    }

    /** Check if ancestorId is an ancestor of nodeId */
    isAncestor(nodeId: string, ancestorId: string): boolean {
        let current = this.nodes.get(nodeId);
        while (current?.parentId) {
            if (current.parentId === ancestorId) return true;
            current = this.nodes.get(current.parentId);
        }
        return false;
    }

    /** Get depth of a block (0 for root) */
    getDepth(id: string): number {
        let depth = 0;
        let current = this.nodes.get(id);
        while (current?.parentId) {
            depth++;
            current = this.nodes.get(current.parentId);
        }
        return depth;
    }

    onChange(callback: () => void): void { this.listeners.push(callback); }
    private notify(): void { for (const cb of this.listeners) cb(); }

    /** Get node by ID */
    getNode(id: string): BlockNode | undefined { return this.nodes.get(id); }
}
