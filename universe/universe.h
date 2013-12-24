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
	kindOf_material_t* materials;

	int n_items;
	kindOf_item_t* items;

	int n_mines;
	kindOf_mine_t* mines;

	int n_buildings;
	kindOf_building_t* buildings;

	// temporarily stores the harvest, transformation
	// and fabrication information
	float* harvestRates;
	int*   rawMaterials;
	float* rawAmounts;
	components_t* item_req;

};

universe_t* universe_init(graphics_t* g);
void        universe_exit(universe_t* u);

void universe_parse(universe_t* u, graphics_t* g, const char* filename);

#endif
