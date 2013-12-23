#ifndef U_UNIVERSE_H
#define U_UNIVERSE_H

typedef struct universe universe_t;

#include "material.h"
#include "item.h"
#include "mine.h"
#include "building.h"
#include "../graphics.h"

struct universe
{
	int n_materials;
	kindOf_material_t materials[26];

	int n_items;
	kindOf_item_t items[5];

	int n_mines;
	kindOf_mine_t mines[8];

	int n_buildings;
	kindOf_building_t buildings[49];
};

universe_t* universe_init(graphics_t* g);
void        universe_exit(universe_t* u);

#endif
