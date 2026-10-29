import assert from 'node:assert/strict';
import test from 'node:test';
import ts from 'typescript';
import { collectForwardedTabProps, collectForwardedOverlayProps, findDomAttribute } from './appearance-dom-contracts.mjs';

function collect(source) {
  return [...collectForwardedTabProps(ts.createSourceFile('fixture.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX))];
}

test('Dialog overlay props prove only forwarded product markers, not overridden library markers', () => {
  const parse = source => [...collectForwardedOverlayProps(ts.createSourceFile('fixture.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX))];
  for (const component of ['Dialog', 'Sheet']) {
    const [node] = parse(`import { ${component} as Surface } from '@alex/ui'; <Surface overlayProps={{'data-alex-product-part': 'lightbox', 'data-alex-component': 'fake', 'data-state': 'fake'}} />;`);
    assert.equal(findDomAttribute(node, 'data-alex-product-part').initializer.text, 'lightbox');
    assert.equal(findDomAttribute(node, 'data-alex-component'), undefined);
    assert.equal(findDomAttribute(node, 'data-state'), undefined);
  }
  for (const source of [
    `import { Dialog } from './fake'; <Dialog overlayProps={{part: 'fake'}} />;`,
    `import { Button } from '@alex/ui'; <Button overlayProps={{part: 'fake'}} />;`,
    `import { Dialog } from '@alex/ui'; <Dialog other={{part: 'fake'}} />;`,
    `import { Dialog } from '@alex/ui'; <Dialog overlayProps={{part: 'fake', ...unknown}} />;`,
    `import { Dialog } from '@alex/ui'; <Dialog overlayProps={{part: 'fake'}} {...unknown} />;`,
    `const unused = {overlayProps: {part: 'fake'}};`,
  ]) assert.equal(parse(source).length, 0, source);
});

test('TabGroup literal and mapped items expose the real native tab attributes', () => {
  for (const items of [
    `[{value: 'models', tabProps: {'data-alex-product-part': 'tab'}}]`,
    `tabs.map(tab => ({value: tab, tabProps: {'data-alex-product-part': 'tab'}}))`,
  ]) {
    const nodes = collect(`import { TabGroup as Tabs } from '@alex/ui'; <Tabs items={${items}} />;`);
    assert.equal(nodes.length, 1);
    assert.equal(findDomAttribute(nodes[0], 'data-alex-product-part').initializer.text, 'tab');
    assert.equal(findDomAttribute(nodes[0], 'missing'), undefined);
  }
});

test('unrelated objects, non-forwarding props and unknown spreads cannot establish DOM contracts', () => {
  for (const source of [
    `const unused = {tabProps: {'data-alex-product-part': 'tab'}};`,
    `import { TabGroup } from './fake'; <TabGroup items={[{tabProps: {part: 'tab'}}]} />;`,
    `import { Button } from '@alex/ui'; <Button items={[{tabProps: {part: 'tab'}}]} />;`,
    `import { TabGroup } from '@alex/ui'; <TabGroup other={[{tabProps: {part: 'tab'}}]} />;`,
    `import { TabGroup } from '@alex/ui'; <TabGroup items={[{tabProps: {part: 'tab'}, ...unknown}]} />;`,
    `import { TabGroup } from '@alex/ui'; <TabGroup items={[{tabProps: {part: 'tab', ...unknown}}]} />;`,
    `import { TabGroup } from '@alex/ui'; <TabGroup items={[{label: {tabProps: {part: 'tab'}}}]} />;`,
  ]) assert.equal(collect(source).length, 0, source);
});

test('component-owned part and value markers cannot be supplied as forwarding evidence', () => {
  const [node] = collect(`import { TabGroup } from '@alex/ui';
    <TabGroup items={[{value: 'models', tabProps: {
      'data-alex-part': 'invented', 'data-alex-value': 'invented',
      'data-alex-product-part': 'tab'
    }}]} />;`);
  assert.equal(findDomAttribute(node, 'data-alex-part'), undefined);
  assert.equal(findDomAttribute(node, 'data-alex-value'), undefined);
  assert.equal(findDomAttribute(node, 'data-alex-product-part').initializer.text, 'tab');
});
