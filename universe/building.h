#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

#include "../graphics.h"
#include "component.h"

struct kindOf_building
{
	const char* name;

	float width;
	float height;
	int   sprite;

	// requisite materials
	components_t build_req;

	// available material
	components_t make_req; // needed
	components_t make_res; // result

	// available items
	int item_n;
	components_t* item_req;
	components_t* item_res;
};

void kindOf_building_init(kindOf_building_t* b, graphics_t* g, const char* name);
void kindOf_building_exit(kindOf_building_t* b);

int kindOf_building_newItem(kindOf_building_t* b);

#endif
