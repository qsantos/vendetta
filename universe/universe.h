#ifndef U_UNIVERSE_H
#define U_UNIVERSE_H

#include "material.h"
#include "mine.h"
#include "building.h"

typedef struct universe universe_t;

struct universe
{
	int n_materials;
	kindOf_material_t materials[26];

	int n_mines;
	kindOf_mine_t mines[8];

	int n_buildings;
	kindOf_building_t buildings[10];
};

void universe_init(universe_t* u);

#endif
