#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

#include <wchar.h>

#include "../graphics.h"
#include "transform.h"

struct kindOf_building
{
	wchar_t* name;

	float width;
	float height;
	int   sprite;
	int   n_sprites;

	int button_sprite;
	int button_index;

	// requisite materials
	transform_t build;

	// available material
	transform_t make;

	// available items
	int n_items;
	transform_t* items;
};

void kindOf_building_init(kindOf_building_t* b);
void kindOf_building_exit(kindOf_building_t* b);

void kindOf_building_sprite(kindOf_building_t* b, graphics_t* g, char* filename, int n_sprites);
void kindOf_building_button(kindOf_building_t* b, graphics_t* g, char* filename, int idx);

int kindOf_building_newItem(kindOf_building_t* b);

#endif
