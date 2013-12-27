#include "overlay.h"

#include <math.h>

#include "../util.h"

#define PANEL_N_COLS 3

overlay_t* overlay_init(void)
{
	overlay_t* o = CALLOC(overlay_t, 1);
	o->selectedBuilding = NULL;

	swbuilding_init (&o->swbuilding);
	swinventory_init(&o->swinventory);
	swskills_init   (&o->swskills);
	swequipment_init(&o->swequipment);

	return o;
}

void overlay_exit(overlay_t* o)
{
	swequipment_exit(&o->swequipment);
	swskills_exit   (&o->swskills);
	swinventory_exit(&o->swinventory);
	swbuilding_exit (&o->swbuilding);

	free(o);
}

void draw_buildPanel(game_t* g)
{
	sfVector2f pos = {0, 0};
	for (int i = 0; i < g->u->n_buildings; i++)
	{
		kindOf_building_t* b = &g->u->buildings[i];

		if (b->button_sprite < 0)
			continue;

		sfSprite* sprite = g->g->sprites[b->button_sprite];

		int ok = transform_check(&b->build, &g->player->inventory);

		sfIntRect rect = {28*b->button_index, 28*ok, 28, 28};
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

void draw_cursor(game_t* g)
{
	sfVector2i cursor = sfMouse_getPositionRenderWindow(g->g->render);
	sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, cursor, g->g->world_view);

	sfIntRect rect = {0, 0, 24, 24};

	kindOf_building_t* b = g->o->selectedBuilding;
	if (b != NULL)
	{
		pos.y += b->height / 2;
		int ok = world_canBuild(g->w, g->player, b, pos.x, pos.y);
		rect.left = 4 * 24;

		pos.x = cursor.x - b->width/2;
		pos.y = cursor.y - b->height/2;

		sfSprite* sprite = g->g->sprites[b->sprite];
		sfIntRect rect = {0, b->height*(b->n_sprites-1), b->width, b->height};
		sfSprite_setTextureRect(sprite, rect);

		sfSprite_setPosition(sprite, pos);
		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);

		sfRectangleShape* shape = NULL;
		if (shape == NULL)
		{
			shape = sfRectangleShape_create();
			sfColor fill = {0, 0, 0, 0};
			sfRectangleShape_setFillColor(shape, fill);
			sfColor outline = {255*(1-ok), 255*ok, 0, 255};
			sfRectangleShape_setOutlineColor(shape, outline);
			sfRectangleShape_setOutlineThickness(shape, 1);
		}

		sfVector2f size = {b->width, b->height};
		sfRectangleShape_setSize(shape, size);
		sfRectangleShape_setPosition(shape, pos);
		sfRenderWindow_drawRectangleShape(g->g->render, shape, NULL);
	}
	else
	{
		object_t* o = world_objectAt(g->w, pos.x, pos.y);
		if (o != NULL)
		{
			if (o->t == O_MINE)
				rect.left = 1 * 24;
			else if (o->t == O_BUILDING)
			{
				building_t* b = (building_t*) o;
				if (b->build_progress == 1)
					rect.left = 7 * 24;
				else
					rect.left = 4 * 24;
			}
		}
	}

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g->g, "cursors.png");
		sprite = g->g->sprites[id];
	}

	sfSprite_setTextureRect(sprite, rect);

	pos.x = cursor.x;
	pos.y = cursor.y;
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
}

void draw_overlay(game_t* g)
{
	draw_buildPanel(g);

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor(text, col);
		sfText_setFont(text, g->g->font);
		sfText_setCharacterSize(text, 15);
	}

	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	float x = 10;
	float y = size.y - 10 - 30 * N_STATUSES;;
	for (int i = 0; i < N_STATUSES; i++)
	{
		float p = g->player->statuses[i] / 20;
		graphics_drawProgressBar(g->g, x, y, 150, 20, p);

		sfText_setUnicodeString(text, (sfUint32*) g->u->statuses[i].name);
		sfVector2f pos = {x+5, y};
		sfText_setPosition(text, pos);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%.0f/%.0f", floor(g->player->statuses[i]), floor(20.));
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		pos.x = x + 140 - rect.width;
		sfText_setPosition(text, pos);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		y += 30;
	}

	swbuilding_draw (&g->o->swbuilding,  g);
	swinventory_draw(&g->o->swinventory, g);
	swskills_draw   (&g->o->swskills,    g);
	swequipment_draw(&g->o->swequipment, g);

	draw_cursor(g);
}

int overlay_catch(game_t* g, float x, float y, int t)
{
	if (swbuilding_catch (&g->o->swbuilding,  g, x, y, t) ||
	    swinventory_catch(&g->o->swinventory, g, x, y, t) ||
	    swskills_catch   (&g->o->swskills,    g, x, y, t) ||
	    swequipment_catch(&g->o->swequipment, g, x, y, t)
	)
		return 1;

	sfFloatRect rect = {0, 0, 28, 28};
	for (int i = 0; i < g->u->n_buildings; i++)
	{
		kindOf_building_t* b = &g->u->buildings[i];

		if (b->button_sprite < 0)
			continue;

		if (sfFloatRect_contains(&rect, x, y))
		{
			if (transform_check(&b->build, &g->player->inventory))
				g->o->selectedBuilding = b;
			return 1;
		}

		rect.left += 28;
		if (rect.left >= 28*PANEL_N_COLS)
		{
			rect.left = 0;
			rect.top += 28;
		}
	}

	kindOf_building_t* b = g->o->selectedBuilding;
	if (b == NULL)
		return 0;

	if (t == sfMouseRight)
	{
		g->o->selectedBuilding = NULL;
	}
	else
	{
		sfVector2i cursor = {x, y};
		sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, cursor, g->g->world_view);
		pos.y += b->height / 2;

		if (!world_canBuild(g->w, g->player, b, pos.x, pos.y))
			return 1;

		transform_apply(&b->build, &g->player->inventory, 1);
		world_addBuilding(g->w, b, pos.x, pos.y);

		g->o->selectedBuilding = NULL;
		return 1;
	}

	return 0;
}
