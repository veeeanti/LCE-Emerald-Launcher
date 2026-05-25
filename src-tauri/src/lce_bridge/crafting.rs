use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LegacyRecipe {
    pub index: u8,
    pub input: Vec<Option<IngredientKey>>,
    pub output: (ItemSpec, u8),
    pub width: u8,
    pub height: u8,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct IngredientKey {
    pub item_id: i16,
    pub data: i16,
    pub accept_any_data: bool,
}

#[derive(Debug, Clone)]
pub struct ItemSpec {
    pub id: i16,
    pub data: i16,
}

pub struct RecipeCatalog {
    pub recipes: Vec<LegacyRecipe>,
    by_index: HashMap<u8, usize>,
}

impl RecipeCatalog {
    pub fn new() -> Self {
        let recipes = build_catalog();
        let mut by_index = HashMap::new();
        for (i, r) in recipes.iter().enumerate() {
            by_index.insert(r.index, i);
        }
        Self { recipes, by_index }
    }

    pub fn recipe_for_index(&self, index: u8) -> Option<&LegacyRecipe> {
        self.by_index.get(&index).map(|&i| &self.recipes[i])
    }
}

fn build_catalog() -> Vec<LegacyRecipe> {
    let mut recipes = Vec::new();
    recipes.push(LegacyRecipe {
        index: 0, width: 1, height: 1,
        input: vec![Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true })],
        output: (ItemSpec { id: 281, data: 0 }, 4),
    });
    recipes.push(LegacyRecipe {
        index: 1, width: 2, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 280, data: 0 }, 4),
    });
    recipes.push(LegacyRecipe {
        index: 2, width: 2, height: 2,
        input: vec![
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 5, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 3, width: 3, height: 3,
        input: vec![
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 4, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 4, width: 2, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 256, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 5, width: 3, height: 2,
        input: vec![
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            None,
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            None,
        ],
        output: (ItemSpec { id: 257, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 6, width: 3, height: 3,
        input: vec![
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            None,
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            None,
            None,
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            None,
        ],
        output: (ItemSpec { id: 258, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 7, width: 2, height: 2,
        input: vec![
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 265, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 266, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 8, width: 1, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 263, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 263, data: 1 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 9, width: 3, height: 2,
        input: vec![
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            None,
            Some(IngredientKey { item_id: 280, data: 0, accept_any_data: true }),
            None,
        ],
        output: (ItemSpec { id: 290, data: 0 }, 1),
    });
    recipes.push(LegacyRecipe {
        index: 10, width: 3, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 336, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 336, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 336, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 30, data: 0 }, 6),
    });
    recipes.push(LegacyRecipe {
        index: 11, width: 1, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 337, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 339, data: 0 }, 4),
    });
    recipes.push(LegacyRecipe {
        index: 12, width: 1, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 338, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 339, data: 0 }, 9),
    });
    recipes.push(LegacyRecipe {
        index: 13, width: 3, height: 3,
        input: vec![
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            None,
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 3, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 17, data: 0 }, 3),
    });
    recipes.push(LegacyRecipe {
        index: 14, width: 2, height: 1,
        input: vec![
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
            Some(IngredientKey { item_id: 5, data: 0, accept_any_data: true }),
        ],
        output: (ItemSpec { id: 280, data: 0 }, 4),
    });
    recipes
}

pub struct InventoryManager {
    pub slots: Vec<LceItemStack>,
    pub catalog: RecipeCatalog,
}

use crate::lce_bridge::lce_packets::LceItemStack;

impl InventoryManager {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            catalog: RecipeCatalog::new(),
        }
    }

    pub fn set_slots(&mut self, slots: Vec<LceItemStack>) {
        self.slots = slots;
    }

    pub fn can_craft(&self, recipe_index: u8, player_inventory: &[LceItemStack]) -> bool {
        if let Some(recipe) = self.catalog.recipe_for_index(recipe_index) {
            let mut needed: HashMap<(i16, i16), u8> = HashMap::new();
            for ing in &recipe.input {
                if let Some(key) = ing {
                    *needed.entry((key.item_id, key.data)).or_insert(0) += 1;
                }
            }
            let mut available: HashMap<(i16, i16), u8> = HashMap::new();
            for item in player_inventory {
                if item.id > 0 && item.count > 0 {
                    *available.entry((item.id, item.damage)).or_insert(0) += item.count;
                }
            }
            for ((id, data), count) in &needed {
                let avail = available.get(&(*id, *data)).copied().unwrap_or(0);
                if avail < *count { return false; }
            }
            true
        } else {
            false
        }
    }
}
