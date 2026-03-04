import { invokeCommand } from "./client";
import type {
  CatalogSummaryDto,
  PackageDetailDto,
  SearchPackagesInput,
  SearchPackagesResult,
  SyncCatalogInput,
  SyncCatalogResult
} from "../types";

export function syncCatalog(input: SyncCatalogInput = {}): Promise<SyncCatalogResult> {
  return invokeCommand("sync_catalog", { input });
}

export function getCatalogSummary(): Promise<CatalogSummaryDto> {
  return invokeCommand("get_catalog_summary");
}

export function searchPackages(input: SearchPackagesInput): Promise<SearchPackagesResult> {
  return invokeCommand("search_packages", { input });
}

export function getPackageDetail(packageId: string): Promise<PackageDetailDto | null> {
  return invokeCommand("get_package_detail", { packageId });
}
