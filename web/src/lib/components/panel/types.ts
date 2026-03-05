export interface PanelSession {
    run_id: string;
    asset_count: number;
    command_count: number;
    disk_size_bytes: number;
    last_modified: string;
}

export interface Asset {
    seq: number;
    input_id: string;
    timestamp: string;
    type: string;
    storage: string;
    path?: string;
    data?: string;
}

export interface PaginatedAssets {
    assets: Asset[];
    total: number;
}
