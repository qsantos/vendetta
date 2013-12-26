#ifndef U_UNIVERSE_H
#define U_UNIVERSE_H

typedef struct universe universe_t;

#include "material.h"
#include "item.h"
#include "mine.h"
#include "building.h"
#include "skill.h"
#include "category.h"
#include "equipment.h"
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

	kindOf_skill_t sskills[N_SPECIAL_SKILLS];

	int n_iskills;
	kindOf_skill_t* iskills;

	kindOf_status_t statuses[N_STATUSES];

	int n_categories;
	kindOf_category_t* categories;

	int n_slots;
	kindOf_slot_t* slots;

	// temporarily stores the harvest, transformation
	// and fabrication information
	transform_t* tmp_materials;
	transform_t* tmp_items;

};

universe_t* universe_init(graphics_t* g);
void        universe_exit(universe_t* u);

void universe_parse(universe_t* u, graphics_t* g, const char* filename);

#endif
