#include "overlay.h"

#include <stdlib.h>
#include <stdio.h>

#include "../util.h"

#define PANEL_N_COLS 3

overlay_t* overlay_init(void)
{
	overlay_t* o = CALLOC(overlay_t, 1);
	o->selectedBuilding = NULL;
	return o;
}

void overlay_exit(overlay_t* o)
{
	free(o);
}

void draw_buildPanel(game_t* g)
{
	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g->g, "buildings.png");
		sprite = g->g->sprites[id];
	}

	sfVector2f pos = {0, 0};
	for (int i = 0; i < g->u->n_buildings; i++)
	{
		int ok = components_check(&g->u->buildings[i].build_req, &g->player->inventory);

		sfIntRect rect = {28*i, 28*ok, 28, 28};
		sfSprite_setTextureRect(sprite, rect);

		sfSprite_setPosition(sprite, pos);

		pos.x += 28;
		if (pos.x >= 28*PANEL_N_COLS)
		{
			pos.x = 0;
			pos.y += 28;
		}

		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
	}
}

void draw_swBuilding(game_t* g)
{
	building_t* b = g->player->inBuilding;
	if (b == NULL)
		return;

	sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {PANEL_N_COLS * 28 + 10, 10};

	sfText_setPosition(text, pos);
	sfText_setString  (text, b->t->name);
	sfRenderWindow_drawText(g->g->render, text, NULL);

	kindOf_building_t* t = b->t;

	pos.y += 20;
	if (t->make_res.n != 0)
	{
		component_t* c = &t->make_res.c[0];
		if (c->is_item)
			exit(1);

		const char* action = t->make_req.n == 0 ? "Harvest" : "Transform to";
		const char* name   = g->u->materials[c->id].name;
		char buffer[1024];
		snprintf(buffer, sizeof(buffer), "%s %s\n", action, name);

		sfText_setPosition(text, pos);
		sfText_setString  (text, buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

	pos.y += 20;
	for (int i = 0; i < t->item_n; i++)
	{
		pos.y += 20;

		components_t* l = &t->item_res[i];
		if (l->n == 0)
			continue;

		component_t* c = &l->c[0];
		if (!c->is_item)
			exit(1);

		const char* name = g->u->items[c->id].name;
		sfText_setPosition(text, pos);
		sfText_setString  (text, name);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

//	sfText_destroy(text); // TODO
}

void draw_cursor(game_t* g)
{
	sfVector2i posi = sfMouse_getPositionRenderWindow(g->g->render);
	sfIntRect rect = {0, 0, 24, 24};

	kindOf_building_t* b = g->o->selectedBuilding;
	if (b != NULL)
	{
		rect.left = 4 * 24;

		int id = b->sprite;
		sfSprite* sprite = g->g->sprites[id];

		sfIntRect rect = sfSprite_getTextureRect(sprite);
		sfVector2f posf = {posi.x - rect.width/2, posi.y - rect.height/2};
		sfSprite_setPosition(sprite, posf);

		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
	}

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g->g, "cursors.png");
		sprite = g->g->sprites[id];
	}

	sfSprite_setTextureRect(sprite, rect);

	sfVector2f posf = {posi.x, posi.y};
	sfSprite_setPosition(sprite, posf);
	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
}

void draw_overlay(game_t* g)
{
	draw_buildPanel(g);
	draw_swBuilding(g);
	draw_cursor(g);
}

int overlay_catch(game_t* g, float x, float y)
{
	int i = y / 28;
	int j = x / 28;
	int id = PANEL_N_COLS*i + j;
	if (j < PANEL_N_COLS && id < g->u->n_buildings)
	{
		if (components_check(&g->u->buildings[i].build_req, &g->player->inventory))
			g->o->selectedBuilding = &g->u->buildings[id];
		return 1;
	}

	kindOf_building_t* b = g->o->selectedBuilding;
	if (b != NULL)
	{
		if (!components_check(&b->build_req, &g->player->inventory))
			return 1;

		components_apply(&b->build_req, &g->player->inventory, -1);

		int id = b->sprite;
		sfSprite* sprite = g->g->sprites[id];

		sfVector2i pix = {x, y};
		sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, pix, g->g->world_view);

		sfIntRect rect = sfSprite_getTextureRect(sprite);
		world_addBuilding(g->w, b, pos.x, pos.y + rect.height/2);

		g->o->selectedBuilding = NULL;
		return 1;
	}

	return 0;
}
