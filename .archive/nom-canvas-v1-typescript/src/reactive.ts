export type ReactiveCallback = (value: unknown) => void;

interface ReactiveNode {
    id: string;
    value: unknown;
    dependencies: Set<string>;
    dependents: Set<string>;
    compute: (() => unknown) | null;
    listeners: ReactiveCallback[];
}

export class ReactiveGraph {
    private nodes: Map<string, ReactiveNode> = new Map();

    /** Create or update a reactive value */
    set(id: string, value: unknown): void {
        const node = this.getOrCreate(id);
        const changed = node.value !== value;
        node.value = value;
        if (changed) this.propagate(id);
    }

    /** Get current value */
    get(id: string): unknown {
        return this.nodes.get(id)?.value;
    }

    /** Define a computed value that depends on other nodes */
    computed(id: string, deps: string[], compute: (...args: unknown[]) => unknown): void {
        const node = this.getOrCreate(id);
        node.dependencies = new Set(deps);
        node.compute = () => compute(...deps.map(d => this.get(d)));
        // Register as dependent
        for (const dep of deps) {
            this.getOrCreate(dep).dependents.add(id);
        }
        // Initial computation
        node.value = node.compute();
    }

    /** Watch a node for changes */
    watch(id: string, callback: ReactiveCallback): () => void {
        const node = this.getOrCreate(id);
        node.listeners.push(callback);
        // Return unwatch function
        return () => {
            node.listeners = node.listeners.filter(cb => cb !== callback);
        };
    }

    /** Remove a node and its dependencies */
    remove(id: string): void {
        const node = this.nodes.get(id);
        if (!node) return;
        // Remove from dependents of dependencies
        for (const dep of node.dependencies) {
            this.nodes.get(dep)?.dependents.delete(id);
        }
        this.nodes.delete(id);
    }

    /** Get all nodes that depend on a given node */
    getDependents(id: string): string[] {
        return Array.from(this.nodes.get(id)?.dependents || []);
    }

    /** Get all nodes that a given node depends on */
    getDependencies(id: string): string[] {
        return Array.from(this.nodes.get(id)?.dependencies || []);
    }

    /** Check for circular dependencies */
    hasCycle(startId: string): boolean {
        const visited = new Set<string>();
        const inStack = new Set<string>();

        const dfs = (id: string): boolean => {
            if (inStack.has(id)) return true; // back edge = cycle
            if (visited.has(id)) return false; // already explored, not on stack = diamond

            visited.add(id);
            inStack.add(id);

            const node = this.nodes.get(id);
            if (node) {
                for (const dep of node.dependents) {
                    if (dfs(dep)) return true;
                }
            }

            inStack.delete(id);
            return false;
        };

        return dfs(startId);
    }

    /** Get all node IDs */
    getAllIds(): string[] {
        return Array.from(this.nodes.keys());
    }

    private getOrCreate(id: string): ReactiveNode {
        if (!this.nodes.has(id)) {
            this.nodes.set(id, {
                id, value: undefined,
                dependencies: new Set(),
                dependents: new Set(),
                compute: null,
                listeners: [],
            });
        }
        return this.nodes.get(id)!;
    }

    private propagate(id: string): void {
        const node = this.nodes.get(id);
        if (!node) return;
        // Notify listeners
        for (const cb of node.listeners) cb(node.value);
        // Recompute dependents
        for (const depId of node.dependents) {
            const dep = this.nodes.get(depId);
            if (dep?.compute) {
                const newValue = dep.compute();
                if (newValue !== dep.value) {
                    dep.value = newValue;
                    this.propagate(depId); // cascade
                }
            }
        }
    }
}
