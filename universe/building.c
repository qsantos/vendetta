#include "building.h"

#include "../util.h"

void kindOf_building_init(kindOf_building_t* b, graphics_t* g, wchar_t* name, char* filename)
{
	b->name = name;

	char s[1024];
	snprintf(s, 1024, "buildings/%s.png", filename);
	free(filename);
	int id = graphics_spriteForImg(g, s);
	b->sprite = id;

	sfIntRect rect = sfSprite_getTextureRect(g->sprites[id]);
	b->width  = rect.width;
	b->height = rect.height;

	b->build_time = 0;

	components_init(&b->build_req);
	components_init(&b->make_res);
	components_init(&b->make_req);

	b->item_n = 0;
	b->item_req = NULL;
	b->item_res = NULL;

	// hard-coded test values
	// 6x Wood -> Hammer
	int n = kindOf_building_newItem(b);
	components_material(&b->item_req[n], 2, 6);
	components_item    (&b->item_res[n], 4, 1);
}

void kindOf_building_exit(kindOf_building_t* b)
{
	components_exit(&b->build_req);
	free(b->name);
}

int kindOf_building_newItem(kindOf_building_t* b)
{
	b->item_req = CREALLOC(b->item_req, components_t, b->item_n+1);
	b->item_res = CREALLOC(b->item_res, components_t, b->item_n+1);

	components_init(&b->item_req[b->item_n]);
	components_init(&b->item_res[b->item_n]);

	return b->item_n++;
}
