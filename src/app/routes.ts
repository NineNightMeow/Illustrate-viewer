import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import HomePage from "../features/home/HomePage.vue";
import GalleryPage from "../features/gallery/GalleryPage.vue";
import LibraryPage from "../features/library/LibraryPage.vue";
import SettingsPage from "../features/settings/SettingsPage.vue";
import ViewerPage from "../features/viewer/ViewerPage.vue";
import FavoritesPage from "../features/favorites/FavoritesPage.vue";
import CollectionsPage from "../features/collections/CollectionsPage.vue";
import CollectionDetailPage from "../features/collections/CollectionDetailPage.vue";
import TagsPage from "../features/tags/TagsPage.vue";
import TagDetailPage from "../features/tags/TagDetailPage.vue";
import SearchPage from "../features/search/SearchPage.vue";
import SmartCollectionsPage from "../features/smartCollections/SmartCollectionsPage.vue";
import SmartCollectionDetailPage from "../features/smartCollections/SmartCollectionDetailPage.vue";

const routes: RouteRecordRaw[] = [
  { path: "/", name: "home", component: HomePage },
  { path: "/library", name: "library", component: LibraryPage },
  { path: "/library/:libraryId", name: "gallery", component: GalleryPage },
  { path: "/viewer/:libraryId", name: "viewer", component: ViewerPage },
  { path: "/favorites", name: "favorites", component: FavoritesPage },
  { path: "/collections", name: "collections", component: CollectionsPage },
  { path: "/collections/:collectionId", name: "collection-detail", component: CollectionDetailPage },
  { path: "/smart-collections", name: "smart-collections", component: SmartCollectionsPage },
  { path: "/smart-collections/:collectionId", name: "smart-collection-detail", component: SmartCollectionDetailPage },
  { path: "/tags", name: "tags", component: TagsPage },
  { path: "/tags/:tagId", name: "tag-detail", component: TagDetailPage },
  { path: "/search", name: "search", component: SearchPage },
  { path: "/settings", name: "settings", component: SettingsPage },
];

export default createRouter({
  history: createWebHistory(),
  routes,
});
