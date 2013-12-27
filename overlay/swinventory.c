#include "swinventory.h"

#include <math.h>

#define PANEL_N_COLS 3

void swinventory_init(swinventory_t* w)
{
	subwindow_init(&w->w, L"Inventory", 1024-SW_WIDTH*2, 0);
}

void swinventory_exit(swinventory_t* w)
{
	subwindow_exit(&w->w);
}

size_t swinventory_materialTooltip(wchar_t* buffer, size_t n, universe_t* u, kindOf_material_t* m)
{
	size_t cur = 0;

	cur += swprintf(buffer+cur, n-cur, L"%ls", m->name);

	if (m->edible)
	{
		cur += swprintf(buffer+cur, n-cur, L"\nedible:");
		for (int i = 0; i < N_STATUSES; i++)
		{
			float b = m->eatBonus[i];
			if (b == 0)
				continue;

			wchar_t* name = u->statuses[i].name;
			cur += swprintf(buffer+cur, n-cur, L"\n%+.1f %ls", b, name);
		}
	}

	return cur;
}

size_t swinventory_itemTooltip(wchar_t* buffer, size_t n, universe_t* u, kindOf_item_t* it)
{
	size_t cur = 0;

	cur += swprintf(buffer+cur, n-cur, L"%ls", it->name);

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
	{
		float b = it->bonus_special[i];
		if (b == 0)
			continue;

		wchar_t* name = u->sskills[i].name;
		cur += swprintf(buffer+cur, n-cur, L"\n%+.0f %ls", b, name);
	}

	for (int i = 0; i < u->n_materials; i++)
	{
		float b = it->bonus_material[i];
		if (b == 0)
			continue;

		wchar_t* name = u->materials[i].skill.name;
		cur += swprintf(buffer+cur, n-cur, L"\n%+f %ls", b, name);
	}

	for (int i = 0; i < u->n_iskills; i++)
	{
		float b = it->bonus_item[i];
		if (b == 0)
			continue;

		wchar_t* name = u->iskills[i].name;
		cur += swprintf(buffer+cur, n-cur, L"\n%+f %ls", b, name);
	}

	return cur;
}

char swinventory_cursor(swinventory_t* w, game_t* g, float x, float y)
{
	if (!w->w.visible)
		return 0;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 30};

	for (int i = 0; i < g->u->n_materials; i++)
	{
		const wchar_t* name = g->u->materials[i].name;
		int amount = floor(g->player->inventory.materials[i]);

		if (amount == 0)
			continue;

		pos.y += 20;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		swinventory_materialTooltip(buffer, 1024, g->u, &g->u->materials[i]);
		graphics_drawTooltip(g->g, x, y, buffer);

		return 1;
	}

	pos.y += 20;

	for (int i = 0; i < g->u->n_items; i++)
	{
		wchar_t* name = g->u->items[i].name;
		int amount = g->player->inventory.items[i];

		if (amount == 0)
			continue;

		pos.y += 20;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		swinventory_itemTooltip(buffer, 1024, g->u, &g->u->items[i]);
		graphics_drawTooltip(g->g, x, y, buffer);

		return 1;
	}

	return 0;
}

void swinventory_draw(swinventory_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 50};

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

char swinventory_catch(swinventory_t* w, game_t* g, float x, float y, int t)
{
	if (!w->w.visible)
		return 0;

	if (t != sfMouseLeft)
		return 0;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 30};

	for (int i = 0; i < g->u->n_materials; i++)
	{
		const wchar_t* name = g->u->materials[i].name;
		int amount = floor(g->player->inventory.materials[i]);

		if (amount == 0)
			continue;

		pos.y += 20;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		kindOf_material_t* t = &g->u->materials[i];
		if (t->edible)
		{
			g->player->inventory.materials[i]--;
			for (int i = 0; i < N_STATUSES; i++)
				g->player->statuses[i] += t->eatBonus[i];
		}

		return 1;
	}

	pos.y += 20;

	for (int i = 0; i < g->u->n_items; i++)
	{
		wchar_t* name = g->u->items[i].name;
		int amount = g->player->inventory.items[i];

		if (amount == 0)
			continue;

		pos.y += 20;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		int cat = g->u->items[i].category;
		for (int j = 0; j < g->u->n_slots; j++)
		{
			if (g->u->slots[j].category == cat && g->player->equipment[j] < 0)
			{
				g->player->equipment[j] = i;
				g->player->inventory.items[i]--;
				break;
			}
		}

		return 1;
	}

	return subwindow_catch(&w->w, g->g, x, y, t);
}
