#include "overlay.h"

#include <math.h>

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

void draw_swInventory(game_t* g)
{
	sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {PANEL_N_COLS * 28 + 310, 10};

	for (int i = 0; i < g->u->n_materials; i++)
	{
		const wchar_t* name = g->u->materials[i].name;
		int amount = floor(g->player->inventory.materials[i]);

		if (amount == 0)
			continue;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		pos.y += 20;
	}

	pos.y += 20;

	for (int i = 0; i < g->u->n_items; i++)
	{
		wchar_t* name = g->u->items[i].name;
		int amount = g->player->inventory.items[i];

		if (amount == 0)
			continue;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		pos.y += 20;
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
	sfText_setUnicodeString(text, (sfUint32*) b->t->name);
	sfRenderWindow_drawText(g->g->render, text, NULL);

	kindOf_building_t* t = b->t;

	pos.y += 20;
	if (t->make_res.n != 0)
	{
		component_t* c = &t->make_res.c[0];
		if (c->is_item)
			exit(1);

		const wchar_t* action = t->make_req.n == 0 ? L"Harvest" : L"Transform to";
		const wchar_t* name   = g->u->materials[c->id].name;
		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls %ls", action, name);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
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

		wchar_t* name = g->u->items[c->id].name;
		wchar_t buffer[1024];
		if (i == b->item_current)
			swprintf(buffer, 1024, L"%ls (%i %%)", name, (int) floor(100*b->item_progress));
		else
			swprintf(buffer, 1024, L"%ls", name);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
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

		sfSprite* sprite = g->g->sprites[b->sprite];
		sfVector2f posf = {posi.x - b->width/2, posi.y - b->height/2};
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
	draw_swInventory(g);
	draw_swBuilding(g);
	draw_cursor(g);
}

int overlay_catch(game_t* g, float x, float y)
{
	if (g->player->inBuilding != NULL)
	{
		building_t* b = g->player->inBuilding;
		kindOf_building_t* t = b->t;

		sfText* text = NULL;
		if (text == NULL)
		{
			text = sfText_create();
			sfText_setFont         (text, g->g->font);
			sfText_setCharacterSize(text, 18);
		}

		sfVector2f pos = {PANEL_N_COLS * 28 + 10, 50};
		for (int i = 0; i < t->item_n; i++)
		{
			pos.y += 20;

			components_t* l = &t->item_res[i];
			if (l->n == 0)
				continue;

			component_t* c = &l->c[0];
			if (!c->is_item)
				exit(1);

			wchar_t* name = g->u->items[c->id].name;
			sfText_setPosition(text, pos);
			sfText_setUnicodeString(text, (sfUint32*) name);

			sfFloatRect rect = sfText_getGlobalBounds(text);
			if (!sfFloatRect_contains(&rect, x, y))
				continue;

			if (components_check(&t->item_req[i], &g->player->inventory))
			{
				b->item_current = i;
				b->item_progress = 0;
			}

			return 1;
		}
	}

	int i = y / 28;
	int j = x / 28;
	int id = PANEL_N_COLS*i + j;
	if (j < PANEL_N_COLS && id < g->u->n_buildings)
	{
		if (components_check(&g->u->buildings[id].build_req, &g->player->inventory))
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
