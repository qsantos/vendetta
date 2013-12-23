#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

#include "../graphics.h"
#include "material_list.h"

struct kindOf_building
{
	const char* name;

	float width;
	float height;
	int   sprite;

	// requisite materials
	material_list_t build_req;

	// available material
	material_list_t make_res; // result
	material_list_t make_req; // needed
};

void kindOf_building_init(kindOf_building_t* b, graphics_t* g, const char* name);
void kindOf_building_exit(kindOf_building_t* b);

#endif
