#include "building.h"

#include <stdio.h>

void kindOf_building_init(kindOf_building_t* b, graphics_t* g, const char* name)
{
	b->name = name;

	char s[1024];
	snprintf(s, 1024, "buildings/%s.png", name);
	int id = graphics_spriteForImg(g, s);
	b->sprite = id;

	sfIntRect rect = sfSprite_getTextureRect(g->sprites[id]);
	b->width  = rect.width;
	b->height = rect.height;

	material_list_init(&b->req);
	material_list_push(&b->req, 2, 5);
}

void kindOf_building_exit(kindOf_building_t* b)
{
	material_list_exit(&b->req);
}
