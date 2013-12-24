#include "building.h"

#include "../util.h"

void kindOf_building_init(kindOf_building_t* b)
{
	b->name = NULL;
	b->width = 0;
	b->height = 0;
	b->sprite = 0;
	b->n_sprites = 0;

	b->button_sprite = -1;
	b->button_index = -1;

	b->build_time = 0;

	components_init(&b->build_req);
	components_init(&b->make_res);
	components_init(&b->make_req);

	b->item_n = 0;
	b->item_req = NULL;
	b->item_res = NULL;
}

void kindOf_building_exit(kindOf_building_t* b)
{
	components_exit(&b->build_req);
	free(b->name);
}

void kindOf_building_sprite(kindOf_building_t* b, graphics_t* g, char* filename, int n_sprites)
{
	char s[1024];
	snprintf(s, 1024, "buildings/%s", filename);
	int id = graphics_spriteForImg(g, s);
	free(filename);

	b->sprite = id;
	b->n_sprites = n_sprites;

	sfIntRect rect = sfSprite_getTextureRect(g->sprites[id]);
	b->width  = rect.width;
	b->height = rect.height / n_sprites;
}

void kindOf_building_button(kindOf_building_t* b, graphics_t* g, char* filename, int idx)
{
	int id = graphics_spriteForImg(g, filename);
	free(filename);

	b->button_sprite = id;
	b->button_index = idx;
}

int kindOf_building_newItem(kindOf_building_t* b)
{
	b->item_req = CREALLOC(b->item_req, components_t, b->item_n+1);
	b->item_res = CREALLOC(b->item_res, components_t, b->item_n+1);

	components_init(&b->item_req[b->item_n]);
	components_init(&b->item_res[b->item_n]);

	return b->item_n++;
}
